use tmi_rs::Connection;

fn main() {
    env_logger::init();
    let connection = Connection::default();
    connection.open().unwrap();
}
