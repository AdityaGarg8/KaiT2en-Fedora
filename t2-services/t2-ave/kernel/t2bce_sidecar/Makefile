modname := t2bce_sidecar
obj-m += $(modname).o

t2bce_sidecar-objs := usb.o session.o video.o bplist.o

# A plain USB driver for the device t2bce_vhci exposes; no BCE core symbols.
ccflags-y += -I$(src)

KVERSION := $(KERNELRELEASE)
ifeq ($(origin KERNELRELEASE), undefined)
KVERSION := $(shell uname -r)
endif

KDIR := /lib/modules/$(KVERSION)/build
MODDEST ?= /lib/modules/$(KVERSION)/updates
PWD := $(shell pwd)

.PHONY: all clean install

all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean

install:
	@test -f t2bce_sidecar.ko || { echo "t2bce_sidecar.ko not found; run make first"; exit 1; }
	install -d -o root -g root -m 0755 $(MODDEST)
	install -o root -g root -m 0644 t2bce_sidecar.ko $(MODDEST)/t2bce_sidecar.ko
	depmod -a $(KVERSION)
