import type {
  ChangeTimeline,
  FollowUp,
  FollowUpAnalysis,
  ReworkAnalysis,
} from '../types'
import { formatDate, formatDelay, formatNumber, shortSha } from '../types'
import { Empty } from './Status'
import { Panel } from './Panel'

function FollowUpRow({ followup }: { followup: FollowUp }) {
  return (
    <div className="cochange-row">
      <div className="cochange-files">
        <span className="sha" title={followup.source_sha}>
          {shortSha(followup.source_sha)}
        </span>
        <span className="cochange-plus">→</span>
        <span className="sha" title={followup.followup_sha}>
          {shortSha(followup.followup_sha)}
        </span>
        <span className="hotspot-path" title={followup.followup_files.join(', ')}>
          {followup.followup_files.slice(0, 2).join(', ')}
          {followup.followup_files.length > 2 &&
            ` +${followup.followup_files.length - 2} more`}
        </span>
      </div>
      <span className="chip">{followup.reason}</span>
      <span className="chip">{followup.evidence_type}</span>
      <span className="seq-delay">{formatDelay(followup.delay_seconds)}</span>
    </div>
  )
}

export function PropagationHistory({
  timeline,
  followups,
  rework,
}: {
  timeline: ChangeTimeline
  followups: FollowUpAnalysis
  rework: ReworkAnalysis
}) {
  const recent = timeline.entries.slice(-10).reverse()

  return (
    <Panel
      id="propagation"
      title="Propagation History"
      hint="Follow-ups and candidate rework · observed history"
    >
      <p className="history-note">
        Historical evidence — later commits linked to earlier ones by deterministic rules
        (same file, co-change history, dependency, fix message). Candidate rework means a
        rule matched; it is not a confirmed defect and not a prediction.
      </p>

      <div className="panel-sub-title">
        Follow-up commits ({formatNumber(followups.total)} detected)
      </div>
      {followups.followups.length === 0 ? (
        <Empty text="No follow-up commits detected in this history sample." />
      ) : (
        followups.followups
          .slice(0, 12)
          .map((followup) => (
            <FollowUpRow
              key={`${followup.source_sha}-${followup.followup_sha}`}
              followup={followup}
            />
          ))
      )}

      <div className="panel-sub-title">
        Candidate rework ({formatNumber(rework.total)} detected)
      </div>
      {rework.events.length === 0 ? (
        <Empty text="No candidate rework events detected in this history sample." />
      ) : (
        rework.events.slice(0, 12).map((event) => (
          <div
            className="cochange-row"
            key={`${event.file}-${event.initial_sha}-${event.rework_sha}-${event.rule}`}
          >
            <div className="cochange-files">
              <span className="hotspot-path" title={event.file}>
                {event.file}
              </span>
            </div>
            <span className="chip chip-warn">{event.rule}</span>
            <span className="seq-delay">{formatDelay(event.delay_seconds)}</span>
          </div>
        ))
      )}
      {rework.events.length > 0 && (
        <p className="evidence">{rework.events[0].evidence}</p>
      )}

      <div className="panel-sub-title">
        Recent timeline ({formatNumber(timeline.total_commits)} commits
        {timeline.truncated ? ', showing latest' : ''})
      </div>
      {recent.length === 0 ? (
        <Empty text="No commits in this history sample." />
      ) : (
        recent.map((entry) => (
          <div className="timeline-row" key={entry.sha}>
            <span className="sha" title={entry.sha}>
              {shortSha(entry.sha)}
            </span>
            <span className="timeline-message" title={entry.message}>
              {entry.message.split('\n')[0] || '(no message)'}
            </span>
            <span className="timeline-meta">
              {formatDate(entry.date)} · {formatNumber(entry.files.length)} files · +
              {formatNumber(entry.additions)}/−{formatNumber(entry.deletions)}
            </span>
          </div>
        ))
      )}
    </Panel>
  )
}
