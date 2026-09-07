cask "vmux" do
  version "0.0.34"
  sha256 "d00702c6521e515b8ffc89678296a4f19d7448b6270394d0dcf8ec781a35936e"

  url "https://github.com/vmux-ai/vmux/releases/download/v0.0.34/Vmux_0.0.34_aarch64.dmg"
  name "Vmux"
  desc "AI-native workspace combining browser and terminal panes"
  homepage "https://vmux.ai/"

  depends_on macos: :ventura

  app "Vmux.app"

  zap trash: [
    "~/Library/Application Support/ai.vmux.desktop",
    "~/Library/Caches/ai.vmux.desktop",
    "~/Library/Preferences/ai.vmux.desktop.plist",
  ]
end
