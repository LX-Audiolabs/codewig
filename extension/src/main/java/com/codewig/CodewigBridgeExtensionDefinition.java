package com.codewig;

import java.util.UUID;

import com.bitwig.extension.api.PlatformType;
import com.bitwig.extension.controller.AutoDetectionMidiPortNamesList;
import com.bitwig.extension.controller.ControllerExtensionDefinition;
import com.bitwig.extension.controller.api.ControllerHost;

/**
 * Metadata for the Codewig Bridge virtual controller (no MIDI hardware).
 * Serves both codewig-cli and codewig-live over TCP+JSON.
 */
public class CodewigBridgeExtensionDefinition extends ControllerExtensionDefinition {
    private static final UUID DRIVER_ID = UUID.fromString("c11a1000-b1a7-4000-8000-c11a1000b147");

    @Override
    public String getName() {
        return "Codewig Bridge";
    }

    @Override
    public String getAuthor() {
        return "Codewig";
    }

    @Override
    public String getVersion() {
        return "0.2.1";
    }

    @Override
    public UUID getId() {
        return DRIVER_ID;
    }

    @Override
    public String getHardwareVendor() {
        return "Codewig";
    }

    @Override
    public String getHardwareModel() {
        return "Bridge";
    }

    @Override
    public int getRequiredAPIVersion() {
        return 18;
    }

    @Override
    public int getNumMidiInPorts() {
        return 0;
    }

    @Override
    public int getNumMidiOutPorts() {
        return 0;
    }

    @Override
    public void listAutoDetectionMidiPortNames(
            final AutoDetectionMidiPortNamesList list,
            final PlatformType platformType) {
        // Virtual controller — add manually in Bitwig Preferences → Controllers
    }

    @Override
    public CodewigBridgeExtension createInstance(final ControllerHost host) {
        return new CodewigBridgeExtension(this, host);
    }
}
