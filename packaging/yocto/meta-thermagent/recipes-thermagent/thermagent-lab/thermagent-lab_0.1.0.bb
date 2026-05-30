SUMMARY = "ThermAgent Lab Python helper tools"
DESCRIPTION = "Policy validation, trace simulation, and workload hint writer for ThermAgent Edge."
LICENSE = "Apache-2.0"
LIC_FILES_CHKSUM = "file://LICENSE;md5=21d8afa213260b159869bd2fef1809dc"

SRC_URI = "file://thermagentctl file://LICENSE"
S = "${WORKDIR}"

inherit allarch python3native

RDEPENDS:${PN} += "python3-core python3-json python3-argparse python3-datetime"

PACKAGE_ARCH = "all"

do_install() {
    install -d ${D}${bindir}
    install -m 0755 ${WORKDIR}/thermagentctl ${D}${bindir}/thermagentctl

    # Bytecode precompilation is optional; it avoids first-run cache writes on read-only-ish images.
    ${PYTHON} -m py_compile ${D}${bindir}/thermagentctl || true
}

FILES:${PN} += "${bindir}/thermagentctl ${bindir}/__pycache__"
