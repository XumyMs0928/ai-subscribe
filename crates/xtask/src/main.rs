mod contracts;

fn main() {
    if let Err(message) = contracts::run_from_args(std::env::args().skip(1)) {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
