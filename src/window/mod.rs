mod imp;

use {
    crate::{
        preferences::PreferencesWindow,
        theme_switcher::ThemeSwitcher,
        ConvertUnits,
        CONFIG
    },
    adw::{
        gtk::{
            self,
            gdk_pixbuf::Pixbuf,
            gio::{self, Cancellable, MemoryInputStream, SettingsBindFlags},
            glib::{self, clone, Object},
        },
        prelude::*,
        subclass::prelude::*,
    },
    fretboard_layout::{Handedness, MultiscaleBuilder, Specs, Units, Variant},
};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, adw::Window,
            gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible,
            gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root,
            gtk::ShortcutManager;
}

impl Window {
    #[must_use]
    pub fn new(app: &crate::Application) -> Self {
        let obj: Self = Object::new(&[("application", app)]).expect("Cannot create GfretWindow");
        obj.connect_signals();
        obj.setup_theme_switcher();
        obj.bind_properties(app);
        obj
    }

    fn bind_properties(&self, app: &crate::Application) {
        let settings = &app.imp().settings;
        settings.bind("fret-count", self, "fret-count")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
        settings.bind("scale", &self.imp().scale.adjustment(), "value")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
        settings.bind("treble-scale", &self.imp().scale_multi.adjustment(), "value")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
        settings.bind("nut-width", &self.imp().nut_width.adjustment(), "value")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
        settings.bind("bridge-spacing", &self.imp().bridge_spacing.adjustment(), "value")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
        settings.bind("perpendicular-fret", &self.imp().perpendicular_fret.adjustment(), "value")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
        settings.bind("variant", &self.imp().variant_list.get(), "selected")
            .flags(SettingsBindFlags::DEFAULT)
            .build();
    }

    fn connect_signals(&self) {
        self.imp().variant_list.connect_selected_notify(
            clone!(@strong self as win => move |list| {
                let set = list.selected() == 1 || list.selected() == 2;
                win.toggle_multi(set);
                win.draw_preview();
            }),
        );
        self.imp()
            .scale
            .connect_value_changed(clone!(@weak self as win => move |_scl| {
                win.draw_preview();
            }));
        self.imp()
            .scale_multi
            .connect_value_changed(clone!(@weak self as win => move |_scl| {
                win.draw_preview();
            }));
        self.imp()
            .nut_width
            .connect_value_changed(clone!(@weak self as win => move |_scl| {
                win.draw_preview();
            }));
        self.imp()
            .bridge_spacing
            .connect_value_changed(clone!(@weak self as win => move |_scl| {
                win.draw_preview();
            }));
        self.imp().perpendicular_fret.connect_value_changed(
            clone!(@weak self as win => move |_scl| {
                win.draw_preview();
            }),
        );
        self.imp()
            .fret_count
            .connect_value_changed(clone!(@weak self as win => move |_scl| {
                win.draw_preview();
            }));
    }

    fn setup_theme_switcher(&self) {
        let pop = self
            .imp()
            .menu_button
            .popover()
            .unwrap()
            .downcast::<gtk::PopoverMenu>()
            .unwrap();
        let switcher = ThemeSwitcher::new();
        pop.add_child(&switcher, "theme");
    }

    fn variant(&self) -> Variant {
        match self.imp().variant_list.selected() {
            1 => MultiscaleBuilder::new()
                .scale(self.imp().scale_multi.value())
                .handedness(Handedness::Right)
                .pfret(self.imp().perpendicular_fret.value())
                .build(),
            2 => MultiscaleBuilder::new()
                .scale(self.imp().scale_multi.value())
                .handedness(Handedness::Left)
                .pfret(self.imp().perpendicular_fret.value())
                .build(),
            _ => Variant::Monoscale,
        }
    }

    /// Takes the data represented by our Gtk widgets and outputs a Specs struct
    /// which will be used by the backend to render the svg image.
    #[allow(clippy::cast_sign_loss)]
    fn specs(&self) -> Specs {
        Specs::builder()
            .scale(self.imp().scale.value())
            .count(self.imp().fret_count.value_as_int() as u32)
            .variant(self.variant())
            .nut(self.imp().nut_width.value())
            .bridge(match CONFIG.try_lock().unwrap().units {
                Units::Metric => self.imp().bridge_spacing.value() + 6.0,
                Units::Imperial => self.imp().bridge_spacing.value() + (6.0 / 20.4),
            })
            .build()
    }

    /// Performs a full render of the svg image without saving to disk, and
    /// refreshes the image preview with the new data.
    pub(crate) fn draw_preview(&self) {
        let cfg = CONFIG.try_lock().unwrap().clone();
        let image = self.specs().create_document(Some(cfg)).to_string();
        let bytes = gtk::glib::Bytes::from_owned(image.into_bytes());
        let stream = MemoryInputStream::from_bytes(&bytes);
        let width = self.imp().image_preview.size(gtk::Orientation::Horizontal);
        let pixbuf =
            Pixbuf::from_stream_at_scale(&stream, width, -1, true, Option::<&Cancellable>::None);
        if let Err(e) = pixbuf {
            eprintln!("{e}");
            return;
        }
        self.imp().image_preview.set_pixbuf(Some(&pixbuf.unwrap()));
        /*if swap {
            if let Ok(mut file) = FILE.try_lock() {
                file.unset_current();
                self.set_window_title(&file);
            }
        }*/
    }

    fn toggle_multi(&self, set: bool) {
        self.imp().scale_multi.set_visible(set);
        self.imp().scale_multi_fine.set_visible(set);
        self.imp().pfret_label.set_visible(set);
        self.imp().perpendicular_fret.set_visible(set);
    }

    pub fn run_preferences(&self) {
        let pwin = PreferencesWindow::new();
        pwin.show();
    }
}

impl ConvertUnits for Window {
    fn to_metric(&self) {
        self.imp().bridge_spacing.to_metric();
        self.imp().nut_width.to_metric();
        self.imp().scale_fine.to_metric();
        self.imp().scale_multi_fine.to_metric();
    }

    fn to_imperial(&self) {
        self.imp().bridge_spacing.to_imperial();
        self.imp().nut_width.to_imperial();
        self.imp().scale_fine.to_imperial();
        self.imp().scale_multi_fine.to_imperial();
    }
}
