use relay::relay;
fn main() {
  relay::initiate_relay().expect("Error while trying to instatiante relay WS");
}
