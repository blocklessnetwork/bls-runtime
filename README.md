# blockless-runtime-environment


## How to build
1. Install the rust with rustup, please visit the site 'https://rustup.rs/'.

2. Use follow command for build the project.
```
$ cargo build
```

## The configure file example

```json
{
    "fs_root_path": "/opt/blockless/app",
    "limited_fuel": 200000000,
    "limited_memory": 20,
    "entry": "/opt/blockless/app/main.wasi",
    "permissions": [
        "https://www.163.com"
    ]
}
```

## How to run the app

```bash
$ cargo run PATH_OF_CONFIG
```
