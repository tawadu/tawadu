#!/usr/bin/env python3
"""Minimal org.kde.StatusNotifierWatcher for headless CI (Atlas's design).

Owns the well-known name and accepts SNI registrations — no rendering. That
is all the app's zbus NameHasOwner probe and the ksni backend need to take
the REAL tray path under xvfb. python3-dbus is in Ubuntu's repos; no glib
main loop needed: Connection.read_write_dispatch pumps method calls.
"""
import dbus
import dbus.service

NAME = "org.kde.StatusNotifierWatcher"
PATH = "/StatusNotifierWatcher"
IFACE = "org.kde.StatusNotifierWatcher"


class Watcher(dbus.service.Object):
    def __init__(self, bus):
        self.items = []
        bus_name = dbus.service.BusName(NAME, bus)
        super().__init__(bus_name, PATH)

    @dbus.service.method(IFACE, in_signature="s", out_signature="")
    def RegisterStatusNotifierItem(self, service):
        if service not in self.items:
            self.items.append(service)
        self.StatusNotifierItemRegistered(service)

    @dbus.service.method(IFACE, in_signature="s", out_signature="")
    def RegisterStatusNotifierHost(self, service):
        self.StatusNotifierHostRegistered()

    @dbus.service.signal(IFACE, signature="s")
    def StatusNotifierItemRegistered(self, service):
        pass

    @dbus.service.signal(IFACE)
    def StatusNotifierHostRegistered(self):
        pass

    @dbus.service.method(dbus.PROPERTIES_IFACE, in_signature="ss", out_signature="v")
    def Get(self, interface, prop):
        return self.GetAll(interface).get(prop)

    @dbus.service.method(dbus.PROPERTIES_IFACE, in_signature="s", out_signature="a{sv}")
    def GetAll(self, interface):
        return {
            "IsStatusNotifierHostRegistered": dbus.Boolean(True),
            "RegisteredStatusNotifierItems": dbus.Array(self.items, signature="s"),
            "ProtocolVersion": dbus.Int32(0),
        }


if __name__ == "__main__":
    bus = dbus.SessionBus()
    Watcher(bus)
    print("sni_watcher: owning", NAME, flush=True)
    while True:
        bus.read_write_dispatch(500)
