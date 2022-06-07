# blockless-runtime-environment


## How to build
1. Install the rust with rustup, please visit the site 'https://rustup.rs/'.

2. Use follow command for build the project.
```
$ cargo build
```

## The example of configure file 

```json
{
    "fs_root_path": "/opt/blockless/app",
    "limited_fuel": 1000,
    "limited_memory": 20,
    "entry": "/opt/blockless/app/main.wasi",
    "permissions": [
        "https://www.163.com"
    ]
}
```

- fs_root_path: the app root file system path, when it opened, the app will use the file system. it's the "/" in app

- limited_fuel: the limited of instructions, in the example the instructions is 1000, if the app is running out of the limited will be interruptted, like follow:

```log
[2022-06-07T22:12:47Z ERROR blockless] All fuel is consumed, the app exited, fuel consumed 2013, Max Fuel is 2000.
```

- limited_memory: the max size of memory, in the example the max memory is 20 pages, 1 page is 64k, so the app only use 20*64k physical memory.

- entry: the entry funcion file of app. please see the example of the app.

- permissions: the resources permissions, app can't use the resources out of the permission list.

## How to run the app

```bash
$ cargo run PATH_OF_CONFIG
```
