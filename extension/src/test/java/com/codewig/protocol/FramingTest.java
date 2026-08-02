package com.codewig.protocol;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import org.junit.jupiter.api.Test;

class FramingTest {

    private static String unframe(final byte[] framed) {
        final ByteBuffer buf = ByteBuffer.wrap(framed).order(ByteOrder.BIG_ENDIAN);
        final int len = buf.getInt();
        assertEquals(framed.length - 4, len, "length header must match remaining bytes");
        final byte[] body = new byte[len];
        buf.get(body);
        return new String(body, StandardCharsets.UTF_8);
    }

    @Test
    void roundtripSimpleJson() {
        final String json = "{\"ok\":true,\"result\":{\"bpm\":120}}";
        final byte[] framed = Framing.frameUtf8(json);
        assertEquals(json, unframe(framed));
    }

    @Test
    void headerIsBigEndianLength() {
        final byte[] framed = Framing.frameUtf8("ab"); // 2 body bytes
        assertEquals(4 + 2, framed.length);
        assertEquals(0, framed[0]);
        assertEquals(0, framed[1]);
        assertEquals(0, framed[2]);
        assertEquals(2, framed[3]);
    }

    @Test
    void emptyPayload() {
        final byte[] framed = Framing.frameUtf8("");
        assertEquals(4, framed.length);
        assertEquals("", unframe(framed));
    }

    @Test
    void multibyteUtf8UsesByteLengthNotCharLength() {
        final String json = "{\"msg\":\"hällö € ♪ 日本語\"}";
        final byte[] framed = Framing.frameUtf8(json);
        final int bodyBytes = json.getBytes(StandardCharsets.UTF_8).length;
        assertEquals(4 + bodyBytes, framed.length);
        // Sanity: multibyte payload is longer in bytes than in chars
        assertTrue(bodyBytes > json.length());
        assertEquals(json, unframe(framed));
    }

    @Test
    void largePayload() {
        final char[] chars = new char[200_000];
        Arrays.fill(chars, 'x');
        final String json = new String(chars);
        final byte[] framed = Framing.frameUtf8(json);
        assertEquals(4 + 200_000, framed.length);
        assertEquals(json, unframe(framed));
    }
}
