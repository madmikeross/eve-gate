use neo4rs::{Graph, query};
use std::sync::Arc;
use reqwest::{Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct System {
    constellation_id: Option<i64>,
    name: Option<String>,
    planets: Option<Vec<Planet>>,
    position: Position,
    security_class: Option<String>,
    security_status: f64,
    star_id: Option<i64>,
    stargates: Option<Vec<i64>>,
    system_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Planet {
    planet_id: i64,
    asteroid_belts: Option<Vec<i64>>,
    moons: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
struct Position {
    x: f64,
    y: f64,
    z: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>>{
    let client = Client::new();
    let system_ids = get_system_ids(&client).await.unwrap();

    let graph = get_graph_client().await;

    let system_pulls: Vec<_> = system_ids
        .iter()
        .map(|&system_id| tokio::spawn(pull_system(client.clone(), graph.clone(), system_id)))
        .collect();

    futures::future::try_join_all(system_pulls).await?;

    Ok(())
}

async fn pull_system(
    client: Client,
    graph: Arc<Graph>,
    system_id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match get_system_details(&client, system_id).await {
        Ok(system_details) => {
            if let Err(err) = save_system_to_neo4j(&graph, &system_details).await {
                eprintln!("Error saving system {}: {}", system_details.system_id, err);
            } else {
                print!("{:?}, ", system_details.system_id);
            }
        }
        Err(err) => {
            eprintln!("Error getting system details for system {}: {}", system_id, err);
        }
    }

    Ok(())
}

async fn get_graph_client() -> Arc<Graph> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let pass = "neo4jneo4j"; // assumes you have accessed via the browser and updated pass
    Arc::new(Graph::new(uri, user, pass).await.unwrap())
}

async fn get_system_details(client: &Client, system_id: i64) -> Result<System, reqwest::Error> {
    let system_detail_url = format!("https://esi.evetech.net/latest/universe/systems/{}", system_id);
    let response = client.get(&system_detail_url).send().await?;
    response.json().await
}

async fn get_system_ids(client: &Client) -> Result<Vec<i64>, reqwest::Error> {
    let systems_url = "https://esi.evetech.net/latest/universe/systems/";
    let response = client.get(systems_url).send().await?;
    response.json().await
}

async fn save_system_to_neo4j(graph: &Arc<Graph>, system: &System) -> Result<(), neo4rs::Error> {
    let create_statement = "
        CREATE (s:System {
            system_id: $system_id,
            name: $name,
            constellation_id: $constellation_id,
            security_status: $security_status,
            star_id: $star_id,
            security_class: $security_class,
            x: $x,
            y: $y,
            z: $z,
            planets: $planets,
            stargates: $stargates
        })";

    let constellation_id = serde_json::to_string(&system.constellation_id).unwrap();
    let planets_json = serde_json::to_string(&system.planets).unwrap();
    let security_class_param = system.security_class.as_ref().map(|s| s.as_str()).unwrap_or("");
    let name_param = system.name.as_ref().map(|s| s.as_str()).unwrap_or("");
    let stargates = serde_json::to_string(&system.stargates).unwrap();
    let star_id = serde_json::to_string(&system.star_id).unwrap();

    graph.run(query(&create_statement)
        .param("system_id", system.system_id)
        .param("name", name_param)
        .param("constellation_id", constellation_id)
        .param("security_status", system.security_status)
        .param("star_id", star_id)
        .param("security_class", security_class_param)
        .param("x", system.position.x)
        .param("y", system.position.y)
        .param("z", system.position.z)
        .param("planets", planets_json)
        .param("stargates", stargates))
        .await?;

    Ok(())
}