# Sample code for libvirt-rust

## run
```
RUST_LOG=info cargo run -- qemu:///system
```

## memo
```
# list VM
virsh --all

# start VM
virsh start libvirt-rs-mewz

# delete VM
virsh undefine libvirt-rs-mewz
```