use std::io::{Read, Write};

use log::{info, trace};

use std::io::ErrorKind;

use super::jtag_driver::JtagDriver;

pub struct RemoteBitBang {
    socket: std::net::TcpListener,
    client: Option<std::net::TcpStream>,
    rcv_buffer: Box<[u8; 1024]>,
    send_buffer: Vec<u8>,
}

impl RemoteBitBang {
    pub fn new(ip: &str, port: u16) -> RemoteBitBang {
        let socket = std::net::TcpListener::bind((ip, port)).unwrap();
        socket.set_nonblocking(true).unwrap();
        RemoteBitBang {
            socket,
            client: None,
            rcv_buffer: Box::new([0; 1024]),
            send_buffer: Vec::new(),
        }
    }
    pub fn tick(&mut self, jtag_driver: &mut JtagDriver) {
        if self.client.is_none() {
            match self.socket.accept() {
                Ok((stream, _)) => {
                    info!(
                        "Remote Bitbang Client connected: {:?}",
                        stream.peer_addr().unwrap()
                    );
                    stream.set_nonblocking(true).unwrap();
                    self.client = Some(stream);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // No client connection available
                }
                Err(e) => {
                    println!("Failed to accept client connection: {:?}", e);
                }
            }
        } else {
            // 读取 client 发送的所有数据
            let mut client = self.client.as_ref().unwrap();
            match client.read(self.rcv_buffer.as_mut()) {
                Ok(n) => {
                    if n > 0 {
                        trace!("Read {} bytes from client", n);
                        for command in self.rcv_buffer.iter().take(n) {
                            match command {
                                b'0' => jtag_driver.set_pins(false, false, false),
                                b'1' => jtag_driver.set_pins(false, false, true),
                                b'2' => jtag_driver.set_pins(false, true, false),
                                b'3' => jtag_driver.set_pins(false, true, true),
                                b'4' => jtag_driver.set_pins(true, false, false),
                                b'5' => jtag_driver.set_pins(true, false, true),
                                b'6' => jtag_driver.set_pins(true, true, false),
                                b'7' => jtag_driver.set_pins(true, true, true),
                                b'R' => self.send_buffer.push(if jtag_driver.get_tdo() {
                                    b'1'
                                } else {
                                    b'0'
                                }),
                                b'Q' => {
                                    info!("Quitting");
                                }
                                b'B' => {}
                                b'b' => {}
                                b'r' => {
                                    jtag_driver.reset();
                                }

                                _ => {
                                    info!("Remote bitbang Unknown command: {}", command);
                                }
                            }
                        }

                        match client.write_all(self.send_buffer.as_slice()) {
                            Ok(_) => {
                                trace!("Wrote {} bytes to client", self.send_buffer.len());
                                self.send_buffer.clear();
                            }
                            Err(e) => {
                                info!("Failed to write to client: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    // println!("{:?}", e);
                }
            }
        }
    }
}
