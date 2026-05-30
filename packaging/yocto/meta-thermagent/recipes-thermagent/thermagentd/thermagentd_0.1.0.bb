SUMMARY = "ThermAgent Edge local thermal and power policy daemon"
DESCRIPTION = "Small Rust daemon for Jetson Nano-class edge-AI power and thermal policy experiments."
HOMEPAGE = "https://github.com/<org>/thermagent"
LICENSE = "Apache-2.0"
LIC_FILES_CHKSUM = "file://LICENSE;md5=21d8afa213260b159869bd2fef1809dc"

SRC_URI = " \
    file://thermagentd \
    file://thermagentd.service \
    file://jetson-nano-lite.policy \
    file://thermagentd.tmpfiles \
"

S = "${WORKDIR}/thermagentd"

inherit cargo systemd

SYSTEMD_SERVICE:${PN} = "thermagentd.service"
SYSTEMD_AUTO_ENABLE:${PN} = "enable"

# The PoC has no third-party crate dependencies, so Cargo.lock is tiny and offline-friendly.
CARGO_SRC_DIR = ""

do_install() {
    install -d ${D}${sbindir}

    if [ -x ${B}/target/${RUST_HOST_SYS}/release/thermagentd ]; then
        install -m 0755 ${B}/target/${RUST_HOST_SYS}/release/thermagentd ${D}${sbindir}/thermagentd
    elif [ -x ${B}/target/release/thermagentd ]; then
        install -m 0755 ${B}/target/release/thermagentd ${D}${sbindir}/thermagentd
    else
        binpath=$(find ${B} ${S} -path '*/release/thermagentd' -type f | head -n 1)
        if [ -z "$binpath" ]; then
            bbfatal "thermagentd binary not found after cargo build"
        fi
        install -m 0755 "$binpath" ${D}${sbindir}/thermagentd
    fi

    install -d ${D}${sysconfdir}/thermagent/policy.d
    install -m 0644 ${WORKDIR}/jetson-nano-lite.policy ${D}${sysconfdir}/thermagent/policy.d/jetson-nano-lite.policy

    install -d ${D}${systemd_system_unitdir}
    install -m 0644 ${WORKDIR}/thermagentd.service ${D}${systemd_system_unitdir}/thermagentd.service

    install -d ${D}${localstatedir}/lib/thermagent
    install -d ${D}${libdir}/tmpfiles.d
    install -m 0644 ${WORKDIR}/thermagentd.tmpfiles ${D}${libdir}/tmpfiles.d/thermagentd.conf
}

FILES:${PN} += " \
    ${sysconfdir}/thermagent \
    ${systemd_system_unitdir}/thermagentd.service \
    ${localstatedir}/lib/thermagent \
    ${libdir}/tmpfiles.d/thermagentd.conf \
"
