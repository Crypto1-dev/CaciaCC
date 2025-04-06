use druid::{AppLauncher, Widget, WindowDesc, Data, Lens, Env, widget::{Label, TextBox, Button, Flex}};
use druid::widget::Label;
use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use rand::rngs::OsRng;
use std::fs::{self, File};
use std::io::Write;
use hex;

#[derive(Clone, Data, Lens)]
struct AppState {
    wallet_name: String,
    public_key: String,
    private_key: String,
}

fn create_wallet(wallet_name: String) -> (String, String) {
    let mut rng = OsRng;
    let keypair = Keypair::generate(&mut rng);
    let public_key = keypair.public;
    let private_key = keypair.secret;

    let public_key_hex = hex::encode(public_key.as_bytes());
    let private_key_hex = hex::encode(private_key.to_bytes());

    // Save to files
    let wallet_dir = "./wallets";
    fs::create_dir_all(wallet_dir).expect("Failed to create wallet directory");

    let public_key_path = format!("{}/{}_public.key", wallet_dir, wallet_name);
    let private_key_path = format!("{}/{}_private.key", wallet_dir, wallet_name);

    let mut public_file = File::create(public_key_path).expect("Failed to create public key file");
    let mut private_file = File::create(private_key_path).expect("Failed to create private key file");

    public_file.write_all(public_key_hex.as_bytes()).expect("Failed to write public key");
    private_file.write_all(private_key_hex.as_bytes()).expect("Failed to write private key");

    (public_key_hex, private_key_hex)
}

fn build_ui() -> impl Widget<AppState> {
    let wallet_name_box = TextBox::new().lens(AppState::wallet_name);
    let public_key_label = Label::new(|data: &AppState, _env: &Env| {
        format!("Public Key: {}", data.public_key)
    });
    let private_key_label = Label::new(|data: &AppState, _env: &Env| {
        format!("Private Key: {}", data.private_key)
    });

    let create_button = Button::new("Create Account").on_click(|_ctx, data: &mut AppState, _env| {
        let (public_key, private_key) = create_wallet(data.wallet_name.clone());
        data.public_key = public_key;
        data.private_key = private_key;
    });

    let layout = Flex::column()
        .with_child(Label::new("Enter Wallet Name"))
        .with_spacer(8.0)
        .with_child(wallet_name_box)
        .with_spacer(8.0)
        .with_child(create_button)
        .with_spacer(8.0)
        .with_child(public_key_label)
        .with_spacer(8.0)
        .with_child(private_key_label);

    layout
}

pub fn run_gui() {
    let main_window = WindowDesc::new(build_ui)
        .title("Cacia Cryptocurrency Wallet")
        .window_size((400.0, 200.0));

    let initial_state = AppState {
        wallet_name: String::new(),
        public_key: String::new(),
        private_key: String::new(),
    };

    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch the application");
}
