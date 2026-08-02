import { useState } from "react";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { SettingsNav } from "./SettingsNav";
import thirdPartyNotices from "../../../THIRD_PARTY_NOTICES.md?raw";
import nodeLicenses from "../../../THIRD_PARTY_LICENSES_NODE.txt?raw";
import rustLicenses from "../../../THIRD_PARTY_LICENSES_RUST.txt?raw";
import windowsLicenses from "../../../THIRD_PARTY_LICENSES_DOTNET.txt?raw";
import packageInfo from "../../../package.json";

const LICENSE_DOCUMENTS = [
  { id: "overview", label: "概要", content: thirdPartyNotices },
  { id: "node", label: "Node.js", content: nodeLicenses },
  { id: "rust", label: "Rust", content: rustLicenses },
  { id: "windows", label: "Windows・音声", content: windowsLicenses },
] as const;

type LicenseDocumentId = (typeof LICENSE_DOCUMENTS)[number]["id"];

export function LicenseSettingsPage() {
  const [selectedId, setSelectedId] = useState<LicenseDocumentId>("overview");
  const selected = LICENSE_DOCUMENTS.find((document) => document.id === selectedId)!;

  return (
    <ThreePaneLayout left={<SettingsNav />}>
      <div className="settings-page settings-page--licenses">
        <section className="settings-section settings-license-page">
          <div className="settings-license-page__header">
            <div>
              <h2 className="settings-section__title">ライセンス</h2>
              <p className="settings-note">
                Inquivoraと、配布物に含まれる第三者ソフトウェアのライセンス・著作権表示です。
              </p>
            </div>
            <span className="settings-license-page__version">Inquivora {packageInfo.version}</span>
          </div>
          <div className="settings-license-tabs" role="tablist" aria-label="ライセンス文書">
            {LICENSE_DOCUMENTS.map((document) => (
              <button
                key={document.id}
                type="button"
                role="tab"
                aria-selected={selectedId === document.id}
                className={`settings-license-tabs__button${selectedId === document.id ? " settings-license-tabs__button--active" : ""}`}
                onClick={() => setSelectedId(document.id)}
              >
                {document.label}
              </button>
            ))}
          </div>
          <pre className="settings-license-document" role="tabpanel" tabIndex={0}>
            {selected.content}
          </pre>
        </section>
      </div>
    </ThreePaneLayout>
  );
}
