
use std::{io::{Write, Read}};

use crate::deploy::{Deploy, DeployConfig, DeploySource, DeployResult, DeployError, CallRemote};



pub struct SSHDeploy {
    session: ssh::Session
}


impl SSHDeploy {
    pub fn connect(host: &str, user: &str) -> Result<SSHDeploy, ssh::Error> {
        let mut session = ssh::Session::new()
            .map_err(|()| ssh::Error::Ssh(String::from("can not create ssh session")))?;

        println!("set_host {}", host);
        session.set_host(host)?;
        println!("set_user {}", user);
        session.set_username(user)?;

        println!("parse_cfg");    
        session.parse_config(None)?;
        println!("connect");
        session.connect()?;
        
        println!("{:?}",session.is_server_known());
        
        session.userauth_publickey_auto(None)?;

        Ok(SSHDeploy{ session: session })
    }
}


impl Deploy for SSHDeploy {
    fn deploy(&mut self, src: DeploySource, conf: DeployConfig) -> DeployResult<()> {
        conf.copy_files(src, &mut |src, dst| {

            let file_name = src.file_name().unwrap();

            let buf = std::fs::read(src)
                .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;
            

            {
                let mut scp = self.session.scp_new(ssh::WRITE,dst)
                    .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;

                scp.init()
                    .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;

                scp.push_file(file_name,buf.len(),0o644)
                    .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;

                scp.write(&buf)
                    .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;
            }


            Ok(())
        })
    }
}

impl CallRemote for SSHDeploy {
    fn call_remote(&mut self, cmd: &[u8]) -> DeployResult<()> {
        let mut s = self.session.channel_new().map_err(|err| DeployError::new_copy_err(Box::new(err)))?;

        s.open_session().map_err(|err| DeployError::new_copy_err(Box::new(err)))?;
        s.request_exec(cmd).map_err(|err| DeployError::new_copy_err(Box::new(err)))?;
        s.send_eof().map_err(|err| DeployError::new_copy_err(Box::new(err)))?;
        let mut buf=Vec::new();
        s.stdout().read_to_end(&mut buf).map_err(|err| DeployError::new_copy_err(Box::new(err)))?;

        println!("{:?}",std::str::from_utf8(&buf).unwrap());
        Ok(())
    }
}