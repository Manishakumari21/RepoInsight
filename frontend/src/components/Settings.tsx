import type { RepositoryInfo } from '../types'
import { ANALYSIS_ENDPOINT } from '../config'
import { Panel } from './Panel'

export function Settings({ repository }: { repository: RepositoryInfo }) {
  return (
    <Panel id="settings" title="Settings" hint="Analysis configuration">
      <div className="settings-list">
        <div className="setting-row">
          <span className="setting-label">Repository</span>
          <span className="setting-value">{repository.name}</span>
        </div>
        <div className="setting-row">
          <span className="setting-label">Default branch</span>
          <span className="setting-value">{repository.default_branch}</span>
        </div>
        <div className="setting-row">
          <span className="setting-label">Analysis endpoint</span>
          <span className="setting-value">{ANALYSIS_ENDPOINT}</span>
        </div>
        <div className="setting-row">
          <span className="setting-label">Data source</span>
          <span className="setting-value">GitHub repository metadata + git history</span>
        </div>
      </div>
    </Panel>
  )
}