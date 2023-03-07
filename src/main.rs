use store::store::Store;


mod store;

#[tokio::main]
async fn main() {
    let store = Store::new();

    println!("{:#?}", store.load().await);
}
