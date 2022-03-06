
use std::{io::{Write, Read}};

use crate::deploy::{Deploy, DeployConfig, DeployResult, DeployError, CallRemote, DeployPaths};



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
        

        session.userauth_kbdint(None)?;

        //session.userauth_publickey_auto(None)?;
        println!("authorized");
        
        Ok(SSHDeploy{ session: session })
    }
}


impl Deploy for SSHDeploy {
    fn deploy(&mut self, src: DeployPaths, conf: DeployConfig) -> DeployResult<DeployPaths> {
        conf.copy_files(src, &mut |src, dst| {


            let dstdir = if dst.is_file() { dst.parent().unwrap() } else { dst };
            let file_name = if dst.is_file() { dst.file_name().unwrap() } else { src.file_name().unwrap() };

            println!("coping: {:?} -> {:?}", src, dstdir.join(file_name));

            let buf = std::fs::read(src)
                .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;
            
            println!("buffer.len: {}", buf.len());

            {
                let mut scp = self.session.scp_new(ssh::WRITE,dstdir)
                    .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;

                scp.init()
                    .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;

                scp.push_file(file_name,buf.len(),0o644)
                    .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;

                scp.write(&buf)
                    .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;
            }

            Ok(dstdir.join(file_name))
        })
    }
}

impl CallRemote for SSHDeploy {
    fn call_remote(&mut self, cmd: &[u8]) -> DeployResult<()> {
        println!("running cmd: {:?}", cmd);
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