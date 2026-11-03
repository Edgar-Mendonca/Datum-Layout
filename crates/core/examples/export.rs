//! Writes every example's exports into a folder: one PCF per line and one for the whole plant,
//! the glTF model and the three CSV lists. Used by `scripts/verify.sh` (Datum Pipe's own import
//! and the glTF validator read them) and for looking at the files by hand.
//! Run: cargo run -p datum-core --example export -- out/
use datum_core::{catalogue::Catalogue, examples, gltf, lists, pcf, route};
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "exports".into()));
    std::fs::create_dir_all(&dir).expect("cannot make the folder");
    let cat = Catalogue::builtin();
    for ex in examples::list() {
        let p = examples::get(ex.id).unwrap();
        let b = route::build(&p, &cat);
        for l in &p.lines {
            let f = dir.join(format!("{}__{}.pcf", ex.id, l.number.replace(['"', '/', '\\'], "in")));
            std::fs::write(f, pcf::write(&p, &cat, Some(l.id)).unwrap()).unwrap();
        }
        std::fs::write(dir.join(format!("{}.pcf", ex.id)), pcf::write(&p, &cat, None).unwrap()).unwrap();
        std::fs::write(dir.join(format!("{}.glb", ex.id)), gltf::glb(&p, &b, 1)).unwrap();
        let l = lists::lists(&p, &cat);
        for k in ["lines", "equipment", "mto"] { std::fs::write(dir.join(format!("{}-{k}.csv", ex.id)), lists::csv(&l, k).unwrap()).unwrap(); }
    }
    println!("{}", dir.display());
}
