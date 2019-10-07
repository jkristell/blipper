use std::env::args;
use std::rc::Rc;
use std::cell::RefCell;
use std::error::Error;

use gio::prelude::*;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Builder, Button, ListStore, Grid, ComboBoxText, IconSize, Label};
use gtk::GridExt;


use infrared_remotes::{StandardButton};

use libblipperhost::link::SerialLink;
use infrared_remotes::extra::RemoteControlData;


mod remotes;
//use crate::remotes::RemoteControlData;

struct BlipperGui {
    link: SerialLink,
    remotes: Vec<RemoteControlData>,
    selected: usize,

    // Widgets
    remotecontrol_grid: Grid,
    statusbar_label: Label,
    info_label: Label,
}

impl BlipperGui {

    fn new(remotes: Vec<RemoteControlData>, grid: Grid, statusbar_label: Label, info_label: Label) -> Self {
        Self {
            link: SerialLink::new(),
            remotes,
            selected: 0,
            remotecontrol_grid: grid,
            statusbar_label,
            info_label,
        }
    }

    fn send_command(&mut self, cmd: u8) {

        let remote = &self.remotes[self.selected];

        let cmd = common::RemoteControlCmd {
            txid: remote.protocol as u8,
            addr: remote.addr,
            cmd: cmd,
        };

        println!("Sending command: {:?}", cmd);
        self.link.send_command(common::Command::RemoteControlSend(cmd));
    }

    fn update_button_grid(self_rc: Rc<RefCell<BlipperGui>>) {

        let gui = self_rc.borrow_mut();

        let button_grid = Grid::new();
        button_grid.set_column_homogeneous(true);
        button_grid.set_column_spacing(10);
        button_grid.set_row_homogeneous(true);
        button_grid.set_row_spacing(10);

        let mapping = &gui.remotes[gui.selected].mapping;

        for (idx, (cmdid, standardbutton)) in mapping.iter().cloned().enumerate() {

            let button = button_from_standardbutton(standardbutton);

            let blippergui = self_rc.clone();
            button.connect_clicked(move |_| {
                let mut bgui = blippergui.borrow_mut();
                bgui.send_command(cmdid);
            });

            button_grid.attach(&button, (idx % 3) as i32, (idx / 3) as i32, 1, 1);
        }

        button_grid.show_all();
        gui.remotecontrol_grid.remove_row(1);
        gui.remotecontrol_grid.attach(&button_grid, 0, 1, 1, 1);
    }

    fn connect(self_rc: Rc<RefCell<BlipperGui>>) {
        let mut gui = self_rc.borrow_mut();

        let res = gui.link.connect("/dev/ttyACM0");
        println!("connect res: {:?}", res);

        match res {
            Ok(_) => {
                gui.statusbar_label.set_markup("Connected to <b>/dev/ttyACM0</b>");

                gui.link.send_command(common::Command::Info);
                let info = gui.link.reply_info();

                if let Ok(info) = info {
                    gui.info_label.set_markup(&format!("{:?}", info));
                } else {
                    println!("Failed to get info");
                }
            },
            Err(err) => gui.statusbar_label.set_markup(&format!("<b>{}</b>", err.description())),
        }
    }
}


fn build_ui(application: &gtk::Application) {
    let glade_src = include_str!("blipper.glade");
    let builder = Builder::new_from_string(glade_src);

    let window: ApplicationWindow = builder.get_object("window1").unwrap();
    let rc_combo: ComboBoxText = builder.get_object("rc_combo").unwrap();
    let remotecontrol_grid: Grid = builder.get_object("remotecontrol_grid").unwrap();
    let connect_button: Button = builder.get_object("connect_button").unwrap();
    let statusbar_label: Label = builder.get_object("statusbar_label").unwrap();
    let info_label: Label = builder.get_object("info_label").unwrap();

    window.set_application(Some(application));

    let model = ListStore::new(&[String::static_type(), gtk::Type::U32,]);

    let remotes = remotes::create_remotes();

    for (idx, remote) in remotes.iter().enumerate() {
        model.set(&model.append(), &[0, 1], &[&remote.model, &(idx as u32)]);
    }

    let blippergui = Rc::new(RefCell::new(BlipperGui::new(remotes, remotecontrol_grid, statusbar_label, info_label)));

    rc_combo.set_model(Some(&model));
    rc_combo.set_active_iter(model.get_iter_first().as_ref());

    BlipperGui::update_button_grid(blippergui.clone());

    let blippergui1 = blippergui.clone();

    rc_combo.connect_changed(move |combo| {
        let active_id = combo.get_active_iter().unwrap();
        let value: u32 = model.get_value(&active_id, 1).get().unwrap();

        // Update the selected remote and update view
        {
            let mut bgui = blippergui1.borrow_mut();
            bgui.selected = value as usize;
        }

        BlipperGui::update_button_grid(blippergui1.clone());
    });

    let blippergui_clone = blippergui.clone();
    connect_button.connect_clicked(move |_button| {
        BlipperGui::connect(blippergui_clone.clone());
    });

    window.show_all();
}

fn main() {
    let application = gtk::Application::new(
        Some("com.github.jkristell.blipper.gui"),
        Default::default(),
    ).expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn button_from_standardbutton(standardbutton: StandardButton) -> Button {
    let label = format!("{:?}", standardbutton);

    if let Some(name) = standardbutton_to_icon_name(standardbutton) {
        return Button::new_from_icon_name(Some(name), IconSize::Button);
    }

    let button = match standardbutton {
        StandardButton::One => Button::new_with_label("1"),
        StandardButton::Two => Button::new_with_label("2"),
        StandardButton::Three => Button::new_with_label("3"),
        StandardButton::Four => Button::new_with_label("4"),
        StandardButton::Five => Button::new_with_label("5"),
        StandardButton::Six => Button::new_with_label("6"),
        StandardButton::Seven => Button::new_with_label("7"),
        StandardButton::Eight => Button::new_with_label("8"),
        StandardButton::Nine => Button::new_with_label("9"),
        _ => Button::new_with_label(&label)
    };

    button
}

fn standardbutton_to_icon_name(standardbutton: StandardButton) -> Option<&'static str> {
    use StandardButton::*;
    match standardbutton {
        Play => Some("media-playback-start"),
        Stop => Some("media-playback-stop"),
        Paus => Some("media-playback-pause"),
        ChannelListNext => Some("go-next"),
        ChannelListPrev => Some("go-previous"),
        VolumeDown => Some("audio-volume-low"),
        VolumeUp => Some("audio-volume-high"),

        _ => None,
    }

}
