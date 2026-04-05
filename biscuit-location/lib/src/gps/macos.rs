use std::sync::mpsc;
use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_core_location::{CLLocation, CLLocationManager, CLLocationManagerDelegate};
use objc2_foundation::{NSArray, NSDate, NSError, NSObject, NSObjectProtocol, NSRunLoop};

use crate::types::Location;

type GpsFix = (f64, f64, Option<f64>);

struct DelegateIvars {
    sender: std::sync::Mutex<Option<mpsc::Sender<Option<GpsFix>>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "BiscuitLocationDelegate"]
    #[ivars = DelegateIvars]
    struct LocationDelegate;

    unsafe impl NSObjectProtocol for LocationDelegate {}

    unsafe impl CLLocationManagerDelegate for LocationDelegate {
        #[unsafe(method(locationManager:didUpdateLocations:))]
        fn location_manager_did_update_locations(
            &self,
            _manager: &CLLocationManager,
            locations: &NSArray<CLLocation>,
        ) {
            if let Some(location) = locations.lastObject() {
                let coord = unsafe { location.coordinate() };
                let accuracy = unsafe { location.horizontalAccuracy() };
                let acc = if accuracy >= 0.0 {
                    Some(accuracy)
                } else {
                    None
                };
                if let Some(sender) = self.ivars().sender.lock().unwrap().take() {
                    let fix: Option<GpsFix> =
                        Some((coord.latitude, coord.longitude, acc));
                    let _ = sender.send(fix);
                }
            }
        }

        #[unsafe(method(locationManager:didFailWithError:))]
        fn location_manager_did_fail_with_error(
            &self,
            _manager: &CLLocationManager,
            _error: &NSError,
        ) {
            if let Some(sender) = self.ivars().sender.lock().unwrap().take() {
                let none: Option<GpsFix> = None;
                let _ = sender.send(none);
            }
        }
    }
);

impl LocationDelegate {
    fn new(sender: mpsc::Sender<Option<GpsFix>>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(DelegateIvars {
            sender: std::sync::Mutex::new(Some(sender)),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub async fn current_fix(timeout: Duration) -> crate::Result<Option<Location>> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let manager = unsafe { CLLocationManager::new() };
        let delegate = LocationDelegate::new(tx);
        let delegate_proto: &ProtocolObject<dyn CLLocationManagerDelegate> =
            ProtocolObject::from_ref(&*delegate);
        unsafe { manager.setDelegate(Some(delegate_proto)) };
        unsafe { manager.requestLocation() };

        let deadline = NSDate::dateWithTimeIntervalSinceNow(timeout.as_secs_f64());
        NSRunLoop::currentRunLoop().runUntilDate(&deadline);
    });

    match rx.recv_timeout(timeout + Duration::from_secs(1)) {
        Ok(Some((lat, lon, acc))) => super::gps_location(lat, lon, acc).map(Some),
        Ok(None) | Err(_) => Ok(None),
    }
}
