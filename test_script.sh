sed -i '' 's/let param_type = self.parse_type()?/let param_type = match self.parse_type() { Some(t) => t, None => { println!("parse_type returned None"); return None; } }/g' zeus_compiler/src/parser.rs
cargo build --release --manifest-path=zeus_compiler/Cargo.toml
