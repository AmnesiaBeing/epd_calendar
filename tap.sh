#!/bin/sh
ip tuntap add name tap99 mode tap user $SUDO_USER
ip link set tap99 up
ip addr add 192.168.69.100/24 dev tap99
ip -6 addr add fe80::100/64 dev tap99
ip -6 addr add fdaa::100/64 dev tap99
ip -6 route add fe80::/64 dev tap99
ip -6 route add fdaa::/64 dev tap99

# Enable IP forwarding
sysctl -w net.ipv4.ip_forward=1

# Enable NAT for the tap0 interface
export DEFAULT_IFACE=$(ip route show default | grep -oP 'dev \K\S+')
iptables -A FORWARD -i tap99 -j ACCEPT
iptables -A FORWARD -o ${DEFAULT_IFACE} -j ACCEPT
iptables -t nat -A POSTROUTING -o ${DEFAULT_IFACE} -j MASQUERADE

# To remove the TUN/TAP interface, run:
# sudo ip link del tap99
