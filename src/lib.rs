use crate::Error::LibLoading;
use std::net::TcpListener;
use std::os::unix::prelude::FromRawFd;

const SD_LISTEN_FDS_START: i32 = 3;

type SdListenFdsFunc = unsafe extern "C" fn(unset_environment: i32) -> i32;

const SD_LISTEN_FDS_FUNC_NAME: &[u8] = b"sd_listen_fds\0";

#[derive(Debug)]
pub enum Error {
	LibLoading(libloading::Error),
	LibLoadingFailedToLoadSystemd(String),
	Systemd(std::io::Error),
}

impl From<libloading::Error> for Error {
	fn from(e: libloading::Error) -> Self {
		Error::LibLoading(e)
	}
}

pub fn systemd_socket_activation() -> Result<Vec<TcpListener>, Error> {
	let nfds = unsafe {
		let systemd_lib = libloading::Library::new("libsystemd.so.0").map_err(|err| match err {
			dlopen_err @ libloading::Error::DlOpen { .. } => {
				Error::LibLoadingFailedToLoadSystemd(dlopen_err.to_string())
			}
			e => LibLoading(e),
		})?;

		let sd_listen_fds: libloading::Symbol<SdListenFdsFunc> =
			systemd_lib.get(SD_LISTEN_FDS_FUNC_NAME)?;

		sd_listen_fds(false as i32)
	};

	if nfds < 0 {
		return Err(Error::Systemd(std::io::Error::from_raw_os_error(nfds)));
	}

	let listeners: Vec<TcpListener> = (SD_LISTEN_FDS_START..(SD_LISTEN_FDS_START + nfds))
		.map(|fd| unsafe { TcpListener::from_raw_fd(fd) })
		.collect();

	Ok(listeners)
}
