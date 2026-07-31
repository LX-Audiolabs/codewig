package com.cliwig;

import com.bitwig.extension.controller.ControllerExtension;
import com.bitwig.extension.controller.api.ControllerHost;
import com.bitwig.extension.controller.api.RemoteConnection;
import com.bitwig.extension.controller.api.RemoteSocket;
import com.bitwig.extension.controller.api.SettableStringValue;
import com.cliwig.bridge.ClipService;
import com.cliwig.bridge.DeviceService;
import com.cliwig.bridge.ParamService;
import com.cliwig.bridge.TrackService;
import com.cliwig.bridge.TransportService;
import com.cliwig.protocol.CommandRouter;
import com.cliwig.protocol.Framing;
import com.cliwig.protocol.Messages;
import com.google.gson.JsonObject;
import com.google.gson.JsonSyntaxException;

/**
 * CLIwig bridge: localhost TCP + JSON → Controller API.
 *
 * Wire (symmetric for our CLI):
 * - Client → Extension: 4-byte BE length + UTF-8 JSON (Bitwig strips length)
 * - Extension → Client: same framing applied manually (Bitwig send() is raw)
 */
public class CliwigExtension extends ControllerExtension {
    public static final int DEFAULT_PORT = 9470;

    private TransportService transportService;
    private TrackService trackService;
    private DeviceService deviceService;
    private ParamService paramService;
    private ClipService clipService;
    private CommandRouter router;
    /** Port we asked for / report to clients (getPort() often returns -1). */
    private int listenPort = DEFAULT_PORT;

    protected CliwigExtension(final CliwigExtensionDefinition definition, final ControllerHost host) {
        super(definition, host);
    }

    @Override
    public void init() {
        final ControllerHost host = getHost();

        final SettableStringValue portText = host.getPreferences().getStringSetting(
                "Port",
                "Network",
                8,
                Integer.toString(DEFAULT_PORT));
        portText.markInterested();

        final int requestedPort = parsePort(portText.get(), DEFAULT_PORT);
        listenPort = requestedPort;

        transportService = new TransportService(host);
        trackService = new TrackService(host);
        deviceService = new DeviceService(host, trackService.getCursorTrack());
        paramService = new ParamService(trackService.getCursorTrack(), deviceService.getCursorDevice());
        // Launcher cursor clip follows the selected slot — used for note editing
        clipService = new ClipService(trackService, host.createLauncherCursorClip(64, 128));

        final RemoteSocket socket = host.createRemoteConnection("CLIwig", requestedPort);
        final int reported = socket.getPort();
        // Bitwig sometimes returns -1 even when bound to the requested port.
        if (reported > 0) {
            listenPort = reported;
        } else {
            listenPort = requestedPort;
            host.println("CLIwig: socket.getPort()=" + reported + ", using configured port " + listenPort);
        }

        router = new CommandRouter(
                transportService, trackService, deviceService, paramService, clipService, listenPort);
        socket.setClientConnectCallback(this::onClientConnected);

        host.println("CLIwig listening on 127.0.0.1:" + listenPort);
        host.showPopupNotification("CLIwig ready on port " + listenPort);
    }

    private static int parsePort(final String raw, final int fallback) {
        if (raw == null || raw.isBlank()) {
            return fallback;
        }
        try {
            final int parsed = Integer.parseInt(raw.trim());
            if (parsed >= 1024 && parsed <= 65535) {
                return parsed;
            }
        } catch (final NumberFormatException ignored) {
            // fall through
        }
        return fallback;
    }

    private void onClientConnected(final RemoteConnection conn) {
        final ControllerHost host = getHost();
        host.println("CLIwig client connected");

        conn.setDisconnectCallback(() -> host.println("CLIwig client disconnected"));

        conn.setReceiveCallback(data -> {
            final String json = Messages.utf8(data);
            String response;
            try {
                final JsonObject req = Messages.parseRequest(json);
                response = router.handle(req);
            } catch (final JsonSyntaxException e) {
                response = Messages.error(null, "BAD_JSON", e.getMessage());
            } catch (final Exception e) {
                response = Messages.error(null, "INTERNAL",
                        e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName());
            }
            try {
                // Frame ourselves — Bitwig does not length-prefix outbound send()
                conn.send(Framing.frameUtf8(response));
            } catch (final Exception e) {
                host.errorln("CLIwig send failed: " + e.getMessage());
            }
        });
    }

    @Override
    public void exit() {
        getHost().showPopupNotification("CLIwig stopped");
    }

    @Override
    public void flush() {
        // no continuous MIDI surface
    }

    public TransportService getTransportService() {
        return transportService;
    }
}
