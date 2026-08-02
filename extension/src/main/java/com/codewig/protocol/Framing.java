package com.codewig.protocol;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;

/**
 * Wire framing for Codewig.
 *
 * Bitwig RemoteConnection: inbound messages are length-prefixed (API strips header).
 * Outbound {@code connection.send()} is raw — we must frame responses ourselves
 * so the CLI can use the same 4-byte big-endian length + UTF-8 body layout both ways.
 */
public final class Framing {
    private Framing() {
    }

    public static byte[] frameUtf8(final String json) {
        final byte[] body = json.getBytes(StandardCharsets.UTF_8);
        final ByteBuffer buf = ByteBuffer.allocate(4 + body.length).order(ByteOrder.BIG_ENDIAN);
        buf.putInt(body.length);
        buf.put(body);
        return buf.array();
    }
}
