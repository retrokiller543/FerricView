use clap::Parser;
use std::{env, path::Path};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, name = "FerricViewer")]
struct Opt {
    path: Option<String>,
    #[clap(short, long)]
    verbose: bool,
    #[clap(short, long)]
    long: bool,
    #[clap(short)]
    recursive: bool,
}

mod files {
    use rayon::prelude::*;
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;
    use std::time::SystemTime;

    #[derive(Debug, Clone)]
    pub enum FileTree {
        Directory {
            name: String,
            children: Vec<FileTree>,
        },
        File(FileInfo),
    }

    #[derive(Debug, Clone)]
    pub struct Objects {
        file_tree: FileTree,
    }

    #[derive(Debug, Clone)]
    pub struct FileInfo {
        path: String,
        file_name: String,
        size: u64,
        _permissions: fs::Permissions,
        _file_type: fs::FileType,
        modified: SystemTime,
        _owner: u32, // On UNIX systems
        _group: u32, // On UNIX systems
    }

    impl Objects {
        pub fn search_recursive(path: &Path) -> Objects {
            let file_tree = Self::get_items_recursive(path);

            Objects { file_tree }
        }

        fn get_items_recursive(path: &Path) -> FileTree {
            let entries: Vec<_> = fs::read_dir(path)
                .expect("Failed to read directory")
                .map(|entry| entry.expect("Failed to read entry"))
                .collect();

            let children: Vec<_> = entries
                .par_iter() // Using rayon's par_iter for parallel processing
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        Some(Self::get_items_recursive(&path))
                    } else if let Some(filename) = path.file_name() {
                        let metadata = entry.metadata().expect("Failed to get metadata");
                        let file_info = FileInfo {
                            path: path.to_str().unwrap().to_string(),
                            file_name: filename.to_str().unwrap().to_string(),
                            size: metadata.len(),
                            _permissions: metadata.permissions(),
                            _file_type: metadata.file_type(),
                            modified: metadata.modified().unwrap(),
                            _owner: metadata.uid(),
                            _group: metadata.gid(),
                        };
                        Some(FileTree::File(file_info))
                    } else {
                        None
                    }
                })
                .collect();

            FileTree::Directory {
                name: path.file_name().unwrap().to_str().unwrap().to_string(),
                children,
            }
        }

        pub fn search(path: &Path) -> Objects {
            let file_tree = Self::get_items(path);

            Objects { file_tree }
        }

        fn get_items(path: &Path) -> FileTree {
            let mut children = Vec::new();

            for entry in fs::read_dir(path).expect("Failed to read directory") {
                let entry = entry.expect("Failed to read entry");
                let path = entry.path();

                if let Ok(metadata) = entry.metadata() {
                    if path.is_dir() {
                        children.push(FileTree::Directory {
                            name: path.file_name().unwrap().to_str().unwrap().to_string(),
                            children: vec![],
                        });
                    } else if let Some(filename) = path.file_name() {
                        let file_info = FileInfo {
                            path: path.to_str().unwrap().to_string(),
                            file_name: filename.to_str().unwrap().to_string(),
                            size: metadata.len(),
                            _permissions: metadata.permissions(),
                            _file_type: metadata.file_type(),
                            modified: metadata.modified().unwrap(),
                            _owner: metadata.uid(),
                            _group: metadata.gid(),
                        };
                        children.push(FileTree::File(file_info));
                    } else {
                        panic!("Failed to get filename");
                    }
                }
            }

            FileTree::Directory {
                name: path.file_name().unwrap().to_str().unwrap().to_string(),
                children,
            }
        }

        pub fn get_tree(&self) -> &FileTree {
            &self.file_tree
        }
    }

    pub fn print_file_tree(
        tree: &FileTree,
        indent: usize,
        last: bool,
        path_so_far: &Vec<bool>,
        color_index: &mut usize,
    ) {
        let colors = [
            "\x1B[31m", "\x1B[32m", "\x1B[33m", "\x1B[34m", "\x1B[35m", "\x1B[36m",
        ];

        let color = colors[*color_index % colors.len()];

        // Prefix symbols
        let prefix = if indent == 0 {
            "".to_string()
        } else if last {
            "└── ".to_string()
        } else {
            "├── ".to_string()
        };

        // Generate the padding
        let padding = path_so_far
            .iter()
            .enumerate()
            .map(|(idx, &last_in_path)| {
                if idx == path_so_far.len() - 1 {
                    // Don't color the current level here, we'll color prefix instead
                    if last_in_path {
                        "    ".to_string() // 4 spaces for completed directories
                    } else {
                        "│   ".to_string() // Vertical line for ongoing directories
                    }
                } else if last_in_path {
                    "    ".to_string() // 4 spaces for completed directories
                } else {
                    "│   ".to_string() // Vertical line for ongoing directories
                }
            })
            .collect::<String>();

        match tree {
            FileTree::Directory { name, children } => {
                // Print directory name with color
                println!("{}{}{}{}", padding, color, prefix, name);
                let len = children.len();
                let mut new_path = path_so_far.clone();
                new_path.push(last);
                *color_index += 1; // Increment the color index for child directories
                for (i, child) in children.iter().enumerate() {
                    print_file_tree(child, indent + 4, i == len - 1, &new_path, color_index);
                }
            }
            FileTree::File(info) => {
                // Print file name without color
                println!("{}{}{}", padding, prefix, info.file_name);
            }
        }
    }

    fn print_file_info(info: &FileInfo) {
        // Just as a basic example, you can add more info as needed
        println!("Path: {}", info.path);
        println!("Size: {} bytes", info.size);
        println!("Last modified: {:?}", info.modified);
    }

    pub fn print_file_tree_long(tree: &FileTree, indent: usize) {
        match tree {
            FileTree::Directory { name, children } => {
                // Color directory names in blue using ANSI escape codes
                println!("{}\x1B[34m[{}]", " ".repeat(indent), name);
                for child in children {
                    print_file_tree_long(child, indent + 4);
                }
                println!("\x1B[0m"); // Reset color
            }
            FileTree::File(info) => {
                // Color file names in green using ANSI escape codes
                println!("{}{}\x1B[32m\x1B[0m", " ".repeat(indent), info.file_name);
                if indent > 0 {
                    print_file_info(info);
                }
            }
        }
    }
}

fn main() {
    let opt = Opt::parse();

    dbg!(opt.clone());

    let tree: files::Objects;

    match opt.path {
        Some(path) => {
            if opt.recursive {
                tree = files::Objects::search_recursive(Path::new(&path));
            } else {
                tree = files::Objects::search(Path::new(&path));
            }

            if opt.long {
                files::print_file_tree_long(tree.get_tree(), 0);
            } else {
                let mut initial_color_index = 0;
                files::print_file_tree(
                    tree.get_tree(),
                    0,
                    false,
                    &Vec::new(),
                    &mut initial_color_index,
                );
            }
        }
        None => {
            if opt.recursive {
                tree = files::Objects::search_recursive(&env::current_dir().unwrap());
            } else {
                tree = files::Objects::search(&env::current_dir().unwrap());
            }

            if opt.long {
                files::print_file_tree_long(tree.get_tree(), 0);
            } else {
                let mut initial_color_index = 0;
                files::print_file_tree(
                    tree.get_tree(),
                    0,
                    false,
                    &Vec::new(),
                    &mut initial_color_index,
                );
            }
        }
    }
}
