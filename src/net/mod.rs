//! Networking primitives
//!
use {MioResult, MioError};
use io::{Io, IoHandle, NonBlock};
use buf::{Buf, MutBuf};
use std::net::{SocketAddr, IpAddr};
use std::os::unix::Fd;

pub mod tcp;
pub mod udp;
pub mod unix;


/*
 *
 * ===== Socket options =====
 *
 */

pub trait Socket : IoHandle {
    fn linger(&self) -> MioResult<usize> {
        let linger = try!(nix::getsockopt(self.fd(), nix::SockLevel::Socket, nix::sockopt::Linger)
            .map_err(MioError::from_nix_error));

        if linger.l_onoff > 0 {
            Ok(linger.l_onoff as usize)
        } else {
            Ok(0)
        }
    }

    fn set_linger(&self, dur_s: usize) -> MioResult<()> {
        let linger = nix::linger {
            l_onoff: (if dur_s > 0 { 1 } else { 0 }) as nix::c_int,
            l_linger: dur_s as nix::c_int
        };

        nix::setsockopt(self.fd(), nix::SockLevel::Socket, nix::sockopt::Linger, &linger)
            .map_err(MioError::from_nix_error)
    }

    fn set_reuseaddr(&self, val: bool) -> MioResult<()> {
        nix::setsockopt(self.fd(), nix::SockLevel::Socket, nix::sockopt::ReuseAddr, val)
            .map_err(MioError::from_nix_error)
    }

    fn set_reuseport(&self, val: bool) -> MioResult<()> {
        nix::setsockopt(self.fd(), nix::SockLevel::Socket, nix::sockopt::ReusePort, val)
            .map_err(MioError::from_nix_error)
    }

    fn set_tcp_nodelay(&self, val: bool) -> MioResult<()> {
        nix::setsockopt(self.fd(), nix::SockLevel::Tcp, nix::sockopt::TcpNoDelay, val)
            .map_err(MioError::from_nix_error)
    }
}

// TODO: Rename -> Multicast
pub trait MulticastSocket : Socket {
    // TODO: Rename -> join_group
    fn join_multicast_group(&self, addr: &IpAddr, interface: Option<&IpAddr>) -> MioResult<()> {
        match *addr {
            IpAddr::V4(ref addr) => {
                // Ensure interface is the correct family
                let interface = match interface {
                    Some(&IpAddr::V4(ref addr)) => Some(nix::Ipv4Addr::from_std(addr)),
                    Some(_) => return Err(MioError::other()),
                    None => None,
                };

                // Create the request
                let req = nix::ip_mreq::new(nix::Ipv4Addr::from_std(addr), interface);

                // Set the socket option
                nix::setsockopt(self.fd(), nix::SockLevel::Ip, nix::sockopt::IpAddMembership, &req)
                    .map_err(MioError::from_nix_error)
            }
            _ => unimplemented!(),
        }
    }

    // TODO: Rename -> leave_group
    fn leave_multicast_group(&self, addr: &IpAddr, interface: Option<&IpAddr>) -> MioResult<()> {
        match *addr {
            IpAddr::V4(ref addr) => {
                // Ensure interface is the correct family
                let interface = match interface {
                    Some(&IpAddr::V4(ref addr)) => Some(nix::Ipv4Addr::from_std(addr)),
                    Some(_) => return Err(MioError::other()),
                    None => None,
                };

                // Create the request
                let req = nix::ip_mreq::new(nix::Ipv4Addr::from_std(addr), interface);

                // Set the socket option
                nix::setsockopt(self.fd(), nix::SockLevel::Ip, nix::sockopt::IpDropMembership, &req)
                    .map_err(MioError::from_nix_error)
            }
            _ => unimplemented!(),
        }
    }

    // TODO: Rename -> set_ttl
    fn set_multicast_ttl(&self, val: u8) -> MioResult<()> {
        nix::setsockopt(self.fd(), nix::SockLevel::Ip, nix::sockopt::IpMulticastTtl, val)
            .map_err(MioError::from_nix_error)
    }
}

// TODO:
//  - Break up into TrySend and TryRecv.
//  - Return the amount read / writen
pub trait UnconnectedSocket {

    fn send_to<B: Buf>(&mut self, buf: &mut B, tgt: &SocketAddr) -> MioResult<NonBlock<()>>;

    fn recv_from<B: MutBuf>(&mut self, buf: &mut B) -> MioResult<NonBlock<SocketAddr>>;
}

/*
 *
 * ====== Re-exporting needed nix types ======
 *
 */

mod nix {
    pub use nix::{
        c_int,
        NixError,
    };
    pub use nix::errno::EINPROGRESS;
    pub use nix::sys::socket::{
        sockopt,
        AddressFamily,
        SockAddr,
        SockType,
        SockLevel,
        InetAddr,
        Ipv4Addr,
        MSG_DONTWAIT,
        SOCK_NONBLOCK,
        SOCK_CLOEXEC,
        accept4,
        bind,
        connect,
        getpeername,
        getsockname,
        getsockopt,
        ip_mreq,
        linger,
        listen,
        recvfrom,
        sendto,
        setsockopt,
        socket,
    };

    pub use nix::unistd::{
        read,
        write
    };
}

fn socket(family: nix::AddressFamily, ty: nix::SockType) -> MioResult<Fd> {
    nix::socket(family, ty, nix::SOCK_NONBLOCK | nix::SOCK_CLOEXEC)
        .map_err(MioError::from_nix_error)
}

fn connect(io: &Io, addr: &nix::SockAddr) -> MioResult<bool> {
    match nix::connect(io.fd(), addr) {
        Ok(_) => Ok(true),
        Err(e) => {
            match e {
                nix::NixError::Sys(nix::EINPROGRESS) => Ok(false),
                _ => Err(MioError::from_nix_error(e))
            }
        }
    }
}

fn bind(io: &Io, addr: &nix::SockAddr) -> MioResult<()> {
    nix::bind(io.fd(), addr)
        .map_err(MioError::from_nix_error)
}

fn listen(io: &Io, backlog: usize) -> MioResult<()> {
    nix::listen(io.fd(), backlog)
        .map_err(MioError::from_nix_error)
}

fn accept(io: &Io) -> MioResult<Fd> {
    nix::accept4(io.fd(), nix::SOCK_NONBLOCK | nix::SOCK_CLOEXEC)
        .map_err(MioError::from_nix_error)
}

// UDP & UDS
#[inline]
fn recvfrom(io: &Io, buf: &mut [u8]) -> MioResult<(usize, nix::SockAddr)> {
    nix::recvfrom(io.fd(), buf)
        .map_err(MioError::from_nix_error)
}

// UDP & UDS
#[inline]
fn sendto(io: &Io, buf: &[u8], target: &nix::SockAddr) -> MioResult<usize> {
    nix::sendto(io.fd(), buf, target, nix::MSG_DONTWAIT)
        .map_err(MioError::from_nix_error)
}

fn getpeername(io: &Io) -> MioResult<nix::SockAddr> {
    nix::getpeername(io.fd())
        .map_err(MioError::from_nix_error)
}

fn getsockname(io: &Io) -> MioResult<nix::SockAddr> {
    nix::getsockname(io.fd())
        .map_err(MioError::from_nix_error)
}

/*
 *
 * ===== Helpers =====
 *
 */

fn to_nix_addr(addr: &SocketAddr) -> nix::SockAddr {
    nix::SockAddr::Inet(nix::InetAddr::from_std(addr))
}

fn to_std_addr(addr: nix::SockAddr) -> SocketAddr {
    match addr {
        nix::SockAddr::Inet(ref addr) => addr.to_std(),
        _ => panic!("unexpected unix socket address"),
    }
}
