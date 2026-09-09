fn main() {
    let path = std::path::Path::new("/Users/marcusmaia/Documents/Desenvolvimento/Mundam/file-samples/Arquivos para testes/Image/gpr/GOPR2657 - cópia.GPR");
    println!("exists: {}", path.exists());
    let file_name = path.file_name().unwrap();
    let parent = path.parent().unwrap();
    let mut found = false;
    if let Ok(mut entries) = std::fs::read_dir(parent) {
        while let Some(Ok(entry)) = entries.next() {
            if entry.file_name() == file_name {
                found = true;
                break;
            }
        }
    }
    println!("found exact: {}", found);
}
