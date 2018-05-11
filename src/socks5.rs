use num::FromPrimitive;

enum_from_primitive! {
    enum AuthenticationMethod {
        Pass = 0,
        GSSAPI = 1,
        UserNamePassword = 2,
        IANAReserved(u8),
        PrivateReserved(u8),
        NoAuth = 255
    }
}

impl From<u8> AuthenticationMethod {
    fn from(d:u8){
        match(d) {
            case 
        }
    }
}

pub struct HelloReqV5 {
    ver: u8,
    nmethods: u8,
    methods: Vec<AuthenticationMethod>
}

pub struct HelloRespV5 {
    ver: u8,
    method: u8
}

enum Commands {
    Connect,
    Bind,
    UdpAssociation
}

enum Address {
    IPv4(Ipv4Addr),
    IPv6(Ipv6Addr),
    Domain(String)
}

pub struct LinkReqV5 {
    ver: u8,
    cmd: u8,
    rsv: u8,
    addr: Address,
    port: u16
}

enum LinkRespType {
    Ok,
    GeneralFailue,
    AccessDenied,
    NetworkUnreachable,
    HostUnreachable,
    ConnectionRefused,
    Timeout,
    UnsupportedCommand,
    UnsupportedAddressType,
    Undefined(u8)
}

pub struct LinkRespV5 {
    ver: u8,
    rep: LinkRespType,
    addr: Address,
    port: u16
}
