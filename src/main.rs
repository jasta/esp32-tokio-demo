use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::timer::EspTaskTimerService;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_sys as _;
use esp_idf_sys::{esp, esp_app_desc, EspError};
use log::info;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// Edit these or provide your own way of provisioning...
const WIFI_SSID: &str = "The password is password";
const WIFI_PASS: &str = "password";

// To test, run `cargo run`, then when the server is up, use `nc -v espressif 12345` from
// a machine on the same Wi-Fi network.
const TCP_LISTENING_PORT: u16 = 12345;

esp_app_desc!();

fn main() -> anyhow::Result<()> {
  esp_idf_sys::link_patches();
  esp_idf_svc::log::EspLogger::initialize_default();

  // eventfd is needed by our mio poll implementation.  Note you should set max_fds
  // higher if you have other code that may need eventfd.
  info!("Setting up eventfd...");
  let config = esp_idf_sys::esp_vfs_eventfd_config_t {
    max_fds: 1,
    ..Default::default()
  };
  esp! { unsafe { esp_idf_sys::esp_vfs_eventfd_register(&config) } }?;

  info!("Setting up board...");
  let peripherals = Peripherals::take().unwrap();
  let sysloop = EspSystemEventLoop::take()?;
  let timer = EspTaskTimerService::new()?;
  let nvs = EspDefaultNvsPartition::take()?;

  info!("Initializing Wi-Fi...");
  let wifi = AsyncWifi::wrap(
    EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
    sysloop,
    timer.clone())?;

  info!("Starting async run loop");
  tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()?
      .block_on(async move {
        let mut wifi_loop = WifiLoop { wifi };
        wifi_loop.configure().await?;
        wifi_loop.initial_connect().await?;

        info!("Preparing to launch echo server...");
        tokio::spawn(echo_server());

        info!("Entering main Wi-Fi run loop...");
        wifi_loop.stay_connected().await
      })?;

  Ok(())
}

pub struct WifiLoop<'a> {
  wifi: AsyncWifi<EspWifi<'a>>,
}

impl<'a> WifiLoop<'a> {
  pub async fn configure(&mut self) -> Result<(), EspError> {
    info!("Setting Wi-Fi credentials...");
    self.wifi.set_configuration(&Configuration::Client(ClientConfiguration {
      ssid: WIFI_SSID.into(),
      password: WIFI_PASS.into(),
      ..Default::default()
    }))?;

    info!("Starting Wi-Fi driver...");
    self.wifi.start().await
  }

  pub async fn initial_connect(&mut self) -> Result<(), EspError> {
    self.do_connect_loop(true).await
  }

  pub async fn stay_connected(mut self) -> Result<(), EspError> {
    self.do_connect_loop(false).await
  }

  async fn do_connect_loop(
    &mut self,
    exit_after_first_connect: bool,
  ) -> Result<(), EspError> {
    let wifi = &mut self.wifi;
    loop {
      // Wait for disconnect before trying to connect again.  This loop ensures
      // we stay connected and is commonly missing from trivial examples as it's
      // way too difficult to showcase the core logic of an example and have
      // a proper Wi-Fi event loop without a robust async runtime.  Fortunately, we can do it
      // now!
      wifi.wifi_wait(|| wifi.is_up(), None).await?;

      info!("Connecting to Wi-Fi...");
      wifi.connect().await?;

      info!("Waiting for association...");
      wifi.ip_wait_while(|| wifi.is_up().map(|s| !s), None).await?;

      if exit_after_first_connect {
        return Ok(());
      }
    }
  }
}

async fn echo_server() -> anyhow::Result<()> {
  let addr = format!("0.0.0.0:{TCP_LISTENING_PORT}");

  info!("Binding to {addr}...");
  let listener = TcpListener::bind(&addr).await?;

  loop {
    info!("Waiting for new connection on socket: {listener:?}");
    let (socket, _) = listener.accept().await?;

    info!("Spawning handle for: {socket:?}...");
    tokio::spawn(async move {
      info!("Spawned handler!");
      let peer = socket.peer_addr();
      if let Err(e) = serve_client(socket).await {
        info!("Got error handling {peer:?}: {e:?}");
      }
    });
  }
}

async fn serve_client(mut stream: TcpStream) -> anyhow::Result<()> {
  info!("Handling {stream:?}...");

  let mut buf = [0u8; 512];
  loop {
    info!("About to read...");
    let n = stream.read(&mut buf).await?;
    info!("Read {n} bytes...");

    if n == 0 {
      break;
    }

    stream.write_all(&buf[0..n]).await?;
    info!("Wrote {n} bytes back...");
  }

  Ok(())
}