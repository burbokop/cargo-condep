
use std::{io::{Write, Read}};

use serde::{Serialize, Deserialize};

use crate::deploy::{Deploy, DeployConfig, DeployResult, DeployError, CallRemote, DeployPaths};



#[derive(Serialize, Deserialize, Debug)]
pub struct SSHUserAndHost {
    pub user: String,
    pub host: String
}

pub struct SSHDeploy {
    session: ssh::Session
}


impl SSHDeploy {
    pub fn connect(user_and_host: &SSHUserAndHost) -> Result<SSHDeploy, ssh::Error> {
        let mut session = ssh::Session::new()
            .map_err(|()| ssh::Error::Ssh(String::from("can not create ssh session")))?;

        session.set_host(user_and_host.host.as_str())?;
        session.set_username(user_and_host.user.as_str())?;
        session.parse_config(None)?;
        session.connect()?;
        session.userauth_kbdint(None)?;
        
        Ok(SSHDeploy{ session: session })
    }
}


impl Deploy for SSHDeploy {
    fn deploy(&mut self, src: DeployPaths, conf: DeployConfig) -> DeployResult<DeployPaths> {
        conf.copy_files(src, &mut |src, dst| {
            let dstdir = if dst.is_file() { dst.parent().unwrap() } else { dst };
            let file_name = if dst.is_file() { dst.file_name().unwrap() } else { src.file_name().unwrap() };

            println!("Coping: {} -> {}", src.to_str().unwrap(), dstdir.join(file_name).to_str().unwrap());

            let buf = std::fs::read(src)
                .map_err(|err| DeployError::new_copy_err(Box::new(err)))?;
            
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
        println!("running cmd: {:?}", String::from_utf8(Vec::from(cmd)));
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