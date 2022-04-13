use std::net::Ipv4Addr;
use crate::router::{KeeneticRouterInterface, RouterInterface};
use redis::{Client as RedisClient, Commands};

mod router;

const KEY: &str = "mac_addresses";

fn main() {
    let pass = std::env::var("PASSWORD").expect("Router admin password must be set");

    let keenetic = KeeneticRouterInterface::new(
        Ipv4Addr::new(192, 168, 1, 1),
        "admin",
        pass,
    );


    loop {
        let result = run(&keenetic) ;
        if result.is_err() {
            println!("{}", result.unwrap_err());
        }
    }
}

fn run(keenetic: &KeeneticRouterInterface) -> Result<(), String> {
    let result = keenetic.get_online_mac_addresses()?;
    let redis = RedisClient::open("redis://127.0.0.1/")
        .map(|a| a.get_connection())
        .map_err(|a| format!("Cannot connect to redis {}", a.to_string()))?;
    let mut redis = redis
        .map_err(|a| format!("Cannot connect to redis {}", a.to_string()))?;

    let _: () = redis.del(KEY).map_err(|a| a.to_string())?;

    for x in result {
        redis.sadd(KEY, x).map_err(|a| a.to_string())?;

    }


    Ok(())
}
