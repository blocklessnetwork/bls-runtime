# Blockless-runtime-environment

![](blockless.png)

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
        "https://blockless-website.vercel.app"
        "file://test.txt"
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

- permissions: the resources permissions, app can't use the resources out of the permission list. if you use the resources that are not in permissions list, the app will get the error code of "Permission Deny". if you panic in your app, you will get the error like follow example.

```log
panic: Permission deny
[2022-06-09T02:12:39Z ERROR blockless] Fuel 137607:200000000. wasm trap: wasm `unreachable` instruction executed
```

for the file permission the url is start with "file://", if you use "file:///", should not work.

## How to run the app

```bash
$ cargo run PATH_OF_CONFIG
```
