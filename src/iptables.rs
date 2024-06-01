use anyhow::{bail, Result};
use iptables::IPTables;

pub struct IpTablesRule {
    iptables: IPTables,
}

impl IpTablesRule {
    pub fn create() -> Result<Self> {
        let iptables = match iptables::new(false) {
            Ok(t) => t,
            Err(e) => {
                bail!("Error using iptables: {}", e);
            }
        };

        if let Err(e) = iptables
            .append(
                "nat",
                "OUTPUT",
                "-p tcp --dport 2050 -j DNAT --to-destination 127.0.0.1:2051",
            )
            .and(iptables.append(
                "nat",
                "OUTPUT",
                "-p tcp --dport 2051 -j DNAT --to-destination :2050",
            ))
        {
            bail!("Error creating iptables rule: {}", e);
        }

        println!("IPTables rule created successfully.");

        Ok(Self { iptables })
    }
}

impl Drop for IpTablesRule {
    fn drop(&mut self) {
        match self
            .iptables
            .delete(
                "nat",
                "OUTPUT",
                "-p tcp --dport 2050 -j DNAT --to-destination 127.0.0.1:2051",
            )
            .and(self.iptables.delete(
                "nat",
                "OUTPUT",
                "-p tcp --dport 2051 -j DNAT --to-destination :2050",
            )) {
            Ok(_) => println!("Successfully removed iptables rule."),
            Err(e) => println!("ERROR: couldn't delete iptables rule: {}", e),
        }
    }
}
