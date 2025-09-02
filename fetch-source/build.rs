use vergen_git2::{Emitter, Git2Builder};

fn main() {
    let git2 = Git2Builder::all_git().expect("Failed to get git info");
    Emitter::default()
        .add_instructions(&git2)
        .expect("Failed to add git instructions")
        .emit()
        .expect("Failed to emit git instructions");
}
