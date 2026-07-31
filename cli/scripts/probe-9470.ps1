$ErrorActionPreference = "Stop"
$c = New-Object System.Net.Sockets.TcpClient
$c.NoDelay = $true
$c.Connect("127.0.0.1", 9470)
Write-Host "connected"
$s = $c.GetStream()
$s.ReadTimeout = 2500
$body = [Text.Encoding]::UTF8.GetBytes('{"id":1,"c":"ping"}')
$lenBytes = [BitConverter]::GetBytes([int]$body.Length)
if ([BitConverter]::IsLittleEndian) { [Array]::Reverse($lenBytes) }
$s.Write($lenBytes, 0, 4)
$s.Write($body, 0, $body.Length)
$s.Flush()
Write-Host "sent framed ping body_len=$($body.Length)"
$hdr = New-Object byte[] 4
$got = 0
while ($got -lt 4) {
    $n = $s.Read($hdr, $got, 4 - $got)
    if ($n -le 0) { throw "EOF reading header" }
    $got += $n
}
Write-Host ("header hex: " + [BitConverter]::ToString($hdr))
Write-Host ("header ascii: " + [Text.Encoding]::ASCII.GetString($hdr))
$be = $hdr.Clone()
if ([BitConverter]::IsLittleEndian) { [Array]::Reverse($be) }
$payloadLen = [BitConverter]::ToInt32($be, 0)
Write-Host "BE length field: $payloadLen"
if ($payloadLen -gt 0 -and $payloadLen -lt 4096) {
    $buf = New-Object byte[] $payloadLen
    $g = 0
    while ($g -lt $payloadLen) {
        $n = $s.Read($buf, $g, $payloadLen - $g)
        if ($n -le 0) { break }
        $g += $n
    }
    Write-Host ("FRAMED OK: " + [Text.Encoding]::UTF8.GetString($buf, 0, $g))
} else {
    Write-Host "UNFRAMED response (extension still old / not reloaded)"
    $rest = New-Object byte[] 512
    $n = $s.Read($rest, 0, 512)
    $all = [Text.Encoding]::UTF8.GetString($hdr) + [Text.Encoding]::UTF8.GetString($rest, 0, [Math]::Max(0, $n))
    Write-Host "RAW: $all"
}
$c.Close()
