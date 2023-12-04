/** URConfig - Useless Rocket CONFIGuration: the general configurations status
 * 
 * It store:
 *  - commandline configuration option
 *  - environment (mostly fallback)
 *  - ServiceConf (see serviceconf.rs)
 * URConfig has utility method to create and get the pool of connection
 * to RabbitMQ service (whose parameters are extracted form cmdline, env)
 */

use crate::serviceconf::ServiceConf;

#[derive(Debug)]
pub enum UCommands {
    Parse,
    Dry,
    Run
}

#[derive(Debug)]
pub struct URConfig {
    pub configfile: String,
    pub debug_level: u8,
    pub command: UCommands,
    pub serviceconf: Option<ServiceConf>,
}

impl URConfig {
    pub async fn parse_configfile(&mut self) {
        let serviceconf = ServiceConf::parse_service_def(&self.configfile).await;
        self.serviceconf = Some(serviceconf);
    }
    pub fn set_serviceconf(&mut self, serviceconf: ServiceConf) {
        self.serviceconf = Some(serviceconf);
    }
    pub fn get_socket(&self) -> Option<&str> {
        if let Some(conf) = &self.serviceconf {
            Some(&conf.socketpath)
        } else {
            None
        }
    }
}