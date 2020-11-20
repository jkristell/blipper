use std::cell::RefCell;
use std::env::args;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::{io, thread, time};

use gdk_pixbuf::Pixbuf;
use gio::prelude::*;
use glib::{self, clone};
use gtk::{MessageType, prelude::*};
use gtk::GridExt;
use gtk::{ApplicationWindow, Builder, Button, ComboBoxText, Grid, Image, Label, ListStore, InfoBar};

use infrared::{
    Button as StandardButton,
    remotes::{std::RemoteControlData}
};

use blipper_utils::{
    decoder::{DecodedCommand, Decoders},
    link::SerialLink,
};

use blipper_protocol as common;

struct TransmitPanel {
    rcselect: ComboBoxText,
    grid: Grid,
}

struct InfoPanel {
    version: Label,
    protocols: Label,
}

struct DecoderPanel {
    start: Button,
    protocol: Label,
    address: Label,
    command: Label,
}

struct BlipperGui {
    arclink: Arc<Mutex<SerialLink>>,
    remotes: Vec<RemoteControlData>,
    selected: usize,
    statusbar_label: Label,
    transmit_panel: TransmitPanel,
    info_panel: InfoPanel,
    decoder_panel: DecoderPanel,
    infobar: InfoBar,
}

impl BlipperGui {
    fn new(
        remotes: Vec<RemoteControlData>,
        statusbar_label: Label,
        transmit_panel: TransmitPanel,
        info_panel: InfoPanel,
        decoder_panel: DecoderPanel,
        infobar: InfoBar,
    ) -> Rc<RefCell<BlipperGui>> {
        let blippergui = BlipperGui {
            arclink: Arc::new(Mutex::new(SerialLink::new())),
            remotes,
            selected: 0,
            statusbar_label,
            transmit_panel,
            info_panel,
            decoder_panel,
            infobar,
        };

        let model = ListStore::new(&[String::static_type(), glib::Type::U32]);
        for (idx, remote) in blippergui.remotes.iter().enumerate() {
            let text = format!("{}", remote.model);
            model.set(&model.append(), &[0, 1], &[&text, &(idx as u32)]);
        }

        {
            let combo = &blippergui.transmit_panel.rcselect;
            combo.set_model(Some(&model));
            combo.set_active_iter(model.get_iter_first().as_ref());
        }

        // Create the refcelled version
        let refcelled = Rc::new(RefCell::new(blippergui));

        BlipperGui::update_button_grid(refcelled.clone());

        // Setup the callbacks
        let refcelled_clone = refcelled.clone();
        {
            let decoder_panel = &refcelled.borrow().decoder_panel;

            decoder_panel.start.connect_clicked(move |_button| {
                BlipperGui::capture_raw(refcelled_clone.clone());
            });
        }

        {
            let combo = &refcelled.borrow().transmit_panel.rcselect;

            let refcelled_clone = refcelled.clone();
            combo.connect_changed(move |combo| {
                let active_id = combo.get_active_iter().unwrap();
                let value: u32 = model.get_value(&active_id, 1).get().unwrap().unwrap();

                // Update the selected remote and update view
                {
                    let mut bgui = refcelled_clone.borrow_mut();
                    bgui.selected = value as usize;
                }

                BlipperGui::update_button_grid(refcelled_clone.clone());
            });
        }

        refcelled
    }

    fn send_command(&mut self, cmd: u8) -> io::Result<()> {
        let remote = &self.remotes[self.selected];

        let cmd = common::RemoteControlCmd {
            pid: remote.protocol as u8,
            addr: remote.addr as u16,
            cmd,
        };

        println!("Sending command: {:?}", cmd);

        self.arclink
            .lock()
            .unwrap()
            .send_command(common::Command::RemoteControlSend(cmd))
    }

    fn update_button_grid(self_rc: Rc<RefCell<BlipperGui>>) {
        let gui = self_rc.borrow_mut();

        let button_grid = Grid::new();
        button_grid.set_column_homogeneous(true);
        button_grid.set_column_spacing(10);
        button_grid.set_row_homogeneous(true);
        button_grid.set_row_spacing(10);

        let mapping = &gui.remotes[gui.selected].mapping;

        for (i, (cmdid, standardbutton)) in mapping.iter().cloned().enumerate() {
            let button = button_from_standardbutton(standardbutton);

            let blippergui = self_rc.clone();
            button.connect_clicked(move |_| {
                let mut bgui = blippergui.borrow_mut();
                let _ = bgui.send_command(cmdid);
            });

            button_grid.attach(&button, (i % 3) as i32, (i / 3) as i32, 1, 1);
        }

        button_grid.show_all();
        gui.transmit_panel.grid.remove_row(1);
        gui.transmit_panel.grid.attach(&button_grid, 0, 1, 1, 1);
    }

    fn capture_raw(self_rc: Rc<RefCell<BlipperGui>>) {
        let gui = self_rc.borrow_mut();
        let mut link = gui.arclink.lock().unwrap();

        link.send_command(common::Command::Capture)
            .map_err(|_err| gui.statusbar_label.set_markup("Error Sending"))
            .ok();
    }

    fn connect(self_rc: Rc<RefCell<BlipperGui>>) {
        let gui = self_rc.borrow_mut();

        let mut link = gui.arclink.lock().unwrap();

        let res = link.connect("/dev/ttyACM0");
        println!("connect res: {:?}", res);

        match res {
            Ok(_) => {
                gui.statusbar_label
                    .set_markup("Connected to <b>/dev/ttyACM0</b>");

                link.send_command(common::Command::Info)
                    .map_err(|_err| gui.statusbar_label.set_markup("Error Sending"))
                    .ok();
            }
            //Err(err) => gui.statusbar_label.set_markup(&format!("<b>{}</b>", err)),
            Err(err) => {
                gui.infobar.set_message_type(MessageType::Error);
                gui.statusbar_label.set_markup(&format!("<b>{}</b>", err));
            }
        }
    }
}

fn build_ui(application: &gtk::Application) {
    let glade_src = include_str!("blipper.glade");
    let builder = Builder::from_string(glade_src);

    let window: ApplicationWindow = builder.get_object("window1").unwrap();

    let connect_button: Button = builder.get_object("connect_button").unwrap();
    let statusbar_label = builder.get_object("statusbar_label").unwrap();
    let infobar = builder.get_object("infobar").unwrap();

    let transmit_panel = TransmitPanel {
        rcselect: builder.get_object("rc_combo").unwrap(),
        grid: builder.get_object("remotecontrol_grid").unwrap(),
    };

    let info_panel = InfoPanel {
        version: builder.get_object("info_version").unwrap(),
        protocols: builder.get_object("info_protocols").unwrap(),
    };

    let decoder_panel = DecoderPanel {
        start: builder.get_object("start_receiver_button").unwrap(),
        protocol: builder.get_object("remote_protocol").unwrap(),
        address: builder.get_object("remote_address").unwrap(),
        command: builder.get_object("remote_command").unwrap(),
    };

    window.set_application(Some(application));

    let remotes = infrared::remotes::std::remotes();
    let blippergui = BlipperGui::new(
        remotes,
        statusbar_label,
        transmit_panel,
        info_panel,
        decoder_panel,
        infobar,
    );

    let link_clone = blippergui.borrow().arclink.clone();

    connect_button.connect_clicked(clone!(@weak blippergui => move |_button| {
        BlipperGui::connect(blippergui);
    }));

    // Create a new sender/receiver pair with default priority
    let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    // Spawn the thread and move the sender in there
    thread::spawn(move || loop {
        thread::sleep(time::Duration::from_millis(10));

        {
            let mut link = link_clone.lock().unwrap();
            let res = link.read_reply();

            if let Ok(reply) = res {
                let _ = sender.send(reply);
            }
        }
    });

    // Attach the receiver to the default main context (None)
    // and on every message update the label accordingly.
    receiver.attach(None, move |reply: common::Reply| {
        let samplerate = 40_000;
        let mut decoder = Decoders;

        match reply {
            common::Reply::CaptureReply { data } => {
                let panel = &blippergui.borrow().decoder_panel;

                let v = data.bufs.concat();
                let s = &v[0..data.len as usize];
                let maybe_cmd: Option<DecodedCommand> = decoder.decode_data(s, samplerate).pop();

                println!("{:?}", maybe_cmd);

                if let Some(button) = maybe_cmd {
                    panel
                        .protocol
                        .set_markup(&format!("<b>Protocol:</b> {:?}", button.kind));
                    panel
                        .address
                        .set_markup(&format!("<b>Address:</b> {:?}", button.address));
                    panel
                        .command
                        .set_markup(&format!("<b>Command:</b> {:?}", button.command));
                }
            }
            common::Reply::Info { info } => {
                let info_panel = &blippergui.borrow().info_panel;

                info_panel
                    .version
                    .set_markup(&format!("<b>Version:</b> {}", info.version));
                info_panel
                    .protocols
                    .set_markup(&format!("<b>Protocols:</b> {}", info.transmitters));

                println!("{:?}", info)
            }
            common::Reply::Ok => println!("Ok"),
            _ => println!("Unhandled reply"),
        }

        glib::Continue(true)
    });

    window.show_all();
}

fn main() {
    env_logger::init();

    let application =
        gtk::Application::new(Some("com.github.jkristell.blipper.gui"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn button_from_standardbutton(standardbutton: StandardButton) -> Button {
    if let Some(name) = standardbutton_to_icon_name(standardbutton) {
        let pixbuf = Pixbuf::from_file_at_scale(&format!("icons/{}.svg", name), 24, 24, true).ok();
        let image = Image::new();
        image.set_from_pixbuf(pixbuf.as_ref());

        let button = Button::new();
        button.set_image(Some(&image));
        return button;
    }

    let label = format!("{:?}", standardbutton);

    let button = match standardbutton {
        StandardButton::Zero => Button::with_label("0"),
        StandardButton::One => Button::with_label("1"),
        StandardButton::Two => Button::with_label("2"),
        StandardButton::Three => Button::with_label("3"),
        StandardButton::Four => Button::with_label("4"),
        StandardButton::Five => Button::with_label("5"),
        StandardButton::Six => Button::with_label("6"),
        StandardButton::Seven => Button::with_label("7"),
        StandardButton::Eight => Button::with_label("8"),
        StandardButton::Nine => Button::with_label("9"),
        _ => Button::with_label(&label),
    };

    button
}

fn standardbutton_to_icon_name(standardbutton: StandardButton) -> Option<&'static str> {
    use StandardButton::*;

    return match standardbutton {
        Power => Some("power_settings_new"),
        Setup => Some("build"),
        Source => Some("input"),
        Up => Some("arrow_drop_up"),
        Down => Some("arrow_drop_down"),
        Left => Some("arrow_left"),
        Right => Some("arrow_right"),
        Time => Some("watch_later"),
        Return => Some("keyboard_return"),
        Stop => Some("stop"),
        Rewind => Some("fast_rewind"),
        Play => Some("play_arrow"),
        Paus => Some("pause"),
        Play_Paus => Some("play_arrow"),
        Forward => Some("fast_forward"),

        Shuffle | Random => Some("shuffle"),
        Repeat => Some("repeat"),

        Next => Some("skip_next"),
        Prev => Some("skip_previous"),

        ChannelListNext => Some("keyboard_arrow_right"),
        ChannelListPrev => Some("keyboard_arrow_left"),

        VolumeUp => Some("volume_up"),
        VolumeDown => Some("volume_down"),
        VolumeMute | Mute => Some("volume_mute"),
        Eq => Some("graphic_eq"),
        Subtitle => Some("subtitles"),
        Info => Some("info"),

        _ => None,
        /*
        Teletext => {}
        ChannelPrev => {}
        ChannelList => {}
        Tools => {}
        Return => {}
        Exit => {}
        Enter => {}
        Red => {}
        Green => {}
        Yellow => {}
        Blue => {}
        Emanual => {}
        PictureSize => {}
        Mode => {}
        U_SD => {}
        Plus => {}
        Minus => {}
        Repeat => {}
        PitchReset => {}
        PitchPlus => {}
        PitchMinus => {}
        Prog => {}
        */
    };
}
