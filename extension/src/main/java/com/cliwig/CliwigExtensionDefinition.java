package com.cliwig;

import java.util.UUID;

import com.bitwig.extension.api.PlatformType;
import com.bitwig.extension.controller.AutoDetectionMidiPortNamesList;
import com.bitwig.extension.controller.ControllerExtensionDefinition;
import com.bitwig.extension.controller.api.ControllerHost;

/**
 * Metadata for the CLIwig virtual controller (no MIDI hardware).
 */
public class CliwigExtensionDefinition extends ControllerExtensionDefinition {
    private static final UUID DRIVER_ID = UUID.fromString("c11a1000-b1a7-4000-8000-c11a1000b147");

    @Override
    public String getName() {
        return "CLIwig";
    }

    @Override
    public String getAuthor() {
        return "CLIwig";
    }

    @Override
    public String getVersion() {
        return "0.1.0";
    }

    @Override
    public UUID getId() {
        return DRIVER_ID;
    }

    @Override
    public String getHardwareVendor() {
        return "CLIwig";
    }

    @Override
    public String getHardwareModel() {
        return "CLIwig Bridge";
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
    public CliwigExtension createInstance(final ControllerHost host) {
        return new CliwigExtension(this, host);
    }
}
