use echo_db::connect;
use echo_db::users::find_user_by_id;

async fn test() {
    let p = connect("").await;

    let pool = p.unwrap_or_else(|e| {
        println!("Error{:?}", e);
        panic!("");
    });

    // if let pool = p.unwrap() {
    let results = find_user_by_id(&pool, "1").await;
    let results = results.unwrap_or_else(|e| {
        println!("Error messages:${:?}", e);
        panic!("Test")
    });

    // println!("{:?}", results);
    // }
}

#[tokio::main]
async fn main() {
    test().await;
}
