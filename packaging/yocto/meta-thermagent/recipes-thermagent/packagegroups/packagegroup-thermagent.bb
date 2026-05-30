SUMMARY = "ThermAgent package group"
LICENSE = "Apache-2.0"

inherit packagegroup

RDEPENDS:${PN} = " \
    thermagentd \
    thermagent-lab \
"
