# myos
A minimal OS written in Rust.  
It is based on [this blog series](https://os.phil-opp.com/).

## Setup

Go to the project's root directory:  
`$ cd myos`  

Use a nightly compiler for the current directory:  
`$ rustup override set nightly`  

Install the rust source code:  
`$ rustup component add rust-src `

Install tools needed for creating a bootable disk image:  
`$ cargo install bootimage`  
`$ rustup component add llvm-tools-preview`

## Build  
`$ cargo xbuild`

## Test
[QUEMU](https://www.qemu.org/)  must be installed and then tests can be run with:  
`$ cargo xtest`  
  
Test results are output to the host systems' standard output. 

## Run

There are two options:

 - Install [QUEMU](https://www.qemu.org/) and boot the disk image in it with:  
   `$ cargo xrun`

  - Write the disk image to an USB stick and boot it on a real machine:  
    `$ dd if=target/x86_64-myos/debug/bootimage-myos.bin of=/dev/sdX && sync`  
    where `sdX` is the device name of your USB stick.