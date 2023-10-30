use std::env;

use virt::connect::Connect;
use virt::domain::Domain;
use virt::error::Error;
use virt::sys;

fn show_hypervisor_info(conn: &Connect) -> Result<(), Error> {
    if let Ok(hv_type) = conn.get_type() {
        if let Ok(mut hv_ver) = conn.get_hyp_version() {
            let major = hv_ver / 1000000;
            hv_ver %= 1000000;
            let minor = hv_ver / 1000;
            let release = hv_ver % 1000;
            log::info!(
                "Hypervisor: '{}' version: {}.{}.{}",
                hv_type,
                major,
                minor,
                release
            );
            return Ok(());
        }
    }
    Err(Error::last_error())
}

fn show_domains(conn: &Connect) -> Result<(), Error> {
    let flags = sys::VIR_CONNECT_LIST_DOMAINS_ACTIVE | sys::VIR_CONNECT_LIST_DOMAINS_INACTIVE;

    if let Ok(num_active_domains) = conn.num_of_domains() {
        if let Ok(num_inactive_domains) = conn.num_of_defined_domains() {
            log::info!(
                "There are {} active and {} inactive domains",
                num_active_domains,
                num_inactive_domains
            );
            /* Return a list of all active and inactive domains. Using this API
             * instead of virConnectListDomains() and virConnectListDefinedDomains()
             * is preferred since it "solves" an inherit race between separated API
             * calls if domains are started or stopped between calls */
            if let Ok(doms) = conn.list_all_domains(flags) {
                for dom in doms {
                    let id = dom.get_id().unwrap_or(0);
                    let name = dom.get_name().unwrap_or_else(|_| String::from("no-name"));
                    let active = dom.is_active().unwrap_or(false);
                    log::info!("ID: {}, Name: {}, Active: {}", id, name, active);
                    if let Ok(dinfo) = dom.get_info() {
                        log::info!("Domain info:");
                        log::info!("    State: {}", dinfo.state);
                        log::info!("    Max Memory: {}", dinfo.max_mem);
                        log::info!("    Memory: {}", dinfo.memory);
                        log::info!("    CPUs: {}", dinfo.nr_virt_cpu);
                        log::info!("    CPU Time: {}", dinfo.cpu_time);
                    }
                }
            }
            return Ok(());
        }
    }
    Err(Error::last_error())
}

fn disconnect(mut conn: Connect) {
    if let Err(e) = conn.close() {
        panic!("Failed to disconnect from hypervisor: {}", e);
    }
    log::info!("Disconnected from hypervisor");
}
fn main() {
    // init logger
    env_logger::init();

    let uri = env::args().nth(1).expect("failed to get uri");
    log::info!("Attempting to connect to hypervisor: '{:?}'", uri);

    let mut conn = match Connect::open(&uri) {
        Ok(c) => c,
        Err(e) => panic!("No connection to hypervisor: {}", e),
    };

    match conn.get_uri() {
        Ok(u) => log::info!("Connected to hypervisor at '{}'", u),
        Err(e) => {
            disconnect(conn);
            panic!("Failed to get URI for hypervisor connection: {}", e);
        }
    };

    if let Err(e) = show_hypervisor_info(&conn) {
        disconnect(conn);
        panic!("Failed to show hypervisor info: {}", e);
    }

    if let Err(e) = show_domains(&conn) {
        disconnect(conn);
        panic!("Failed to show domains info: {}", e);
    }

    let name = "libvirt-rs-mewz";
    if let Ok(mut dom) = Domain::lookup_by_name(&conn, name) {
        log::info!("already defined qemu domain");
        let _ = dom.destroy();
        let _ = dom.undefine();
        log::info!("destroy and undefine qemu domain");
        assert_eq!(Ok(()), dom.free());
        assert_eq!(Ok(0), conn.close());

    } else {
        log::info!("define qemu domain");
        /*
        const run_cmd_str = &[_][]const u8{
        "qemu-system-x86_64",
        "-kernel",
        "zig-out/bin/mewz.elf",
        "-m",
        "512",
        "-device",
        "virtio-net,netdev=net0,disable-legacy=on,disable-modern=off",
        "-netdev",
        "user,id=net0,hostfwd=tcp:127.0.0.1:20022-:22,hostfwd=tcp:127.0.0.1:20080-:80",
        "-no-shutdown",
        "-no-reboot",
        "-serial",
        "mon:stdio",
        "-nographic",
    };
        */
        let xml = format!(
            "<domain type=\"qemu\">
		         <name>{}</name>
                         <memory unit=\"KiB\">524288</memory>
                         <vcpu>2</vcpu>
                         <os>
                           <type arch='x86_64' machine='pc'>hvm</type>
                           <boot dev='hd'/>
                           <kernel>/home/ainno/Projects/mew/zig-out/bin/mewz.elf</kernel>
                         </os>
                         <features>
                            <acpi/>
                            <apic/>
                            <pae/>
                         </features>
                         <on_poweroff>destroy</on_poweroff>
                         <on_reboot>restart</on_reboot>
                         <on_crash>restart</on_crash>
                         <devices>
                            <emulator>/usr/bin/qemu-system-x86_64</emulator>
                            <interface type='network'>
                                <mac address='52:54:0:12:34:56'/>
                                <source network='default'/>
                                <model type='virtio'/>
                            </interface>
                            <graphics type='vnc' port='-1' autoport='yes'/>
                            <serial type='pty'>
                                <target port='0'/>
                            </serial>
                         </devices>
                         <qemu:arg value='-no-shutdown'/>
                         <qemu:arg value='-no-reboot'/>
                         <qemu:arg value='-nographic'/>
                       </domain>",
            name
        );
        let result: Result<Domain, Error> = Domain::define_xml(&conn, &xml);
        result.expect("failed to define_xml");
        log::info!("Successfully define qemu domain");
    }
}
