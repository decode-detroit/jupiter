# Jupiter
#### Player and Score Management

Manage players and scores for interactive puzzle games with realtime data backup.

## Getting Started

If you're on a 64-bit GNU/Linux system or 64-bit Raspberry Pi, you can use the the [binary releases here](https://github.com/decode-detroit/jupiter/releases) and move on to installing Jupiter's sister programs below.

If you're on Windows, we can likely provide you with a binary (but don't need it ourselves, so we have not compiled one). If you're on Mac, help us produce working binaries! We don't have an Apple device to compile them.

## Compile From Source (Cross-Platform)

If you would like to contribute to Jupiter, or if you are on Windows or Mac, you'll need to compile from source. Start with these prerequisites.

### Prerequisites

You'll need Rust to compile and run Jupier.

* Installation of Rust: https://www.rust-lang.org/

Follow the directions to download and install Rust tools before you proceed.

### Compiling

Once you have installed the prerequities, clone or download this repository. Compile and run the program using Cargo (included with Rust):
```
cargo run
```

This will take several minutes to download all the components. You'll be left with a running Jupiter instance with an example configuration loaded. You can use
```
cargo run
```

to run Jupiter again (it will not recompile this time). This is a debug version (larger file, but otherwise perfectly functional).

To compile a finished copy for deployment, use
```
cargo build --release
```

The completed binary will be located in the automatically generated "target/release" folder with the name "jupiter".

## Installing Extras

Extras! Everyone loves extras. To take advantage of all Jupiter's features, you'll need **Minerva**.

* Sister program **Minerva** controls game state.
* **Redis** provides real-time crash recovery.

You'll need to install these tools on whichever computers you would like to **run** Jupiter, although Minerva can be run on a separate computer instead if you prefer.

### Minerva for Game Management

Jupiter does not manage the game state itself. Instead, the game is managed by Minerva, a separate program which is developed concurrently. The two projects are separate to improve reliability, reusability, and security.

### Redis for Instant Recovery

The most up-to-date instructions for installing Redis can be found here: https://redis.io/.

The default configuration should work just fine for most purposes. Jupiter will update the settings to make sure every change is written to the disk.

## Raspberry Pi-like Systems (ARM)

# OUT OF DATE

The sections below are copied from Minerva and need to be updated for Jupiter.

It's possible to run Minerva on less-capible systems! For example, a Raspberry Pi 4 can manage most of the tasks of a full computer (video is a bit touchy).

Take careful notes of the steps to
* cross-compile Minerva, and
* setup your Raspberry Pi host to run Minerva

Note: These instructions are written for *compiling* the software on Ubuntu 22.04.

### Cross-Compiling To Raspbian (armhf, 32bit)

To cross-compile, install the correct rust target and install the linker.
```
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf
```
You'll also need to add the armhf architecture to dpkg.
```
sudo dpkg --add-architecture armhf
```
And add these sources to the end of /etc/apt/sources.list.
```
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ jammy main restricted
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ jammy-updates main restricted
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ jammy universe
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ jammy-updates universe
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ jammy multiverse
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ jammy-updates multiverse
```
Make sure to add `[arch=amd64]` to the other sources while you're at it.

Install the dev packages for the new architecture.
```
sudo apt update
sudo apt install libssl-dev:armhf
```

Compile the program using the special armhf build target:
```
env PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig/ cargo build_armhf
```

### Cross-Compiling To Raspbian (aarch64/arm64,64bit)

To cross-compile, install the correct rust target and install the linker.
```
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu 
```
You'll also need to add the arm64 architecture to dpkg.
```
sudo dpkg --add-architecture arm64
```
And add these sources to the end of /etc/apt/sources.list (or if also using 32 bit, combine the two like ```[arch=armhf,arm64]```).
```
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ jammy main restricted
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ jammy-updates main restricted
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ jammy universe
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ jammy-updates universe
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ jammy multiverse
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ jammy-updates multiverse
```
Make sure to add `[arch=amd64]` to the other sources while you're at it.

Install the dev packages for the new architecture.
```
sudo apt update
sudo apt install libssl-dev:arm64
```

Compile the program using the special arm64 build target:
```
env PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig/ cargo build_arm64
```

#### Prepare Your Raspberry Pi

In addition to any packages above, you need to cross compile [Apollo](https://github.com/decode-detroit/apollo) and enable the corresponding changes to the Raspberry Pi listed there for video playback.

Hardware decoding works well for videos up to 1080p at 30 fps. There is a short delay when switching between playing videos, but there is no delay when playing a new video after the first has stopped.

## License

This project is licensed under the GNU GPL Version 3 - see the [LICENSE](LICENSE) file for details

Thanks to all the wonderful free and open source people out there who have made this project possible, especially Mozilla et al. for a beautiful language, the folks at Arduino for the ubiquitous microcontroller platform, and the team at Adafruit for their tireless committment to open source hardware.
