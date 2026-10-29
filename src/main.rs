fn main() {
    if let Err(error) = commander_wolf::run() {
        eprintln!("cwolf: {error}");
        std::process::exit(1);
    }
}
