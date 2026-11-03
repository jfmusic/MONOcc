#!/usr/bin/env bash
set -euo pipefail
export DOTNET_CLI_TELEMETRY_OPTOUT=1
linux_root="$(cd "$(dirname "$0")" && pwd)"
dotnet run --project "$linux_root/../windows/Monocc.Tests/Monocc.Tests.csproj" -c Release
dotnet publish "$linux_root/Monocc.Linux/Monocc.Linux.csproj" -c Release -r linux-x64 --self-contained true -o "$linux_root/dist/publish"
python3 "$linux_root/package_deb.py" "$linux_root/dist/publish" "$linux_root/dist/MONOcc-0.2.0-Linux-amd64-preview.deb"
