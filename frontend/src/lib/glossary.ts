export interface GlossaryTerm {
  term: string
  definition: string
}

export const GLOSSARY: GlossaryTerm[] = [
  {
    term: 'Churn',
    definition:
      'Total added plus deleted lines for a file across the analyzed history. High churn means the file is rewritten often.',
  },
  {
    term: 'Revision',
    definition:
      'One recorded change to a file — the number of analyzed commits that touched it.',
  },
  {
    term: 'Coupling',
    definition:
      'How strongly two files tend to change together, based on repository history. Strong coupling may signal a hidden relationship.',
  },
  {
    term: 'Co-change',
    definition:
      'A commit that modified two files at once. Repeated co-changes are the raw signal behind coupling.',
  },
  {
    term: 'Temporal relationship',
    definition:
      'A pattern where one file tends to change shortly after another, inside the analysis time window.',
  },
  {
    term: 'Prediction',
    definition:
      'The model’s estimate that changing a file will likely need follow-up fixes, learned from past repository behavior.',
  },
  {
    term: 'Confidence',
    definition:
      'How confident the model is in a prediction, from the predicted probability. Not a guarantee.',
  },
  {
    term: 'Evidence',
    definition:
      'Observed repository facts (history, structure) behind a prediction — distinct from the prediction itself.',
  },
  {
    term: 'Historical example',
    definition:
      'A real past commit that resembles the predicted situation. What happened, not what will happen.',
  },
  {
    term: 'Dependency',
    definition:
      'A structural link between files, such as an import, extracted from source code.',
  },
  {
    term: 'Complexity',
    definition:
      'Cyclomatic complexity: roughly, the number of independent paths through a file’s code. Higher means harder to change safely.',
  },
  {
    term: 'Hotspot',
    definition:
      'A file combining high complexity, churn, or coupling — a candidate for closer inspection, not a verdict.',
  },
]

export function glossaryDefinition(term: string): string | null {
  const found = GLOSSARY.find(
    (entry) => entry.term.toLowerCase() === term.toLowerCase(),
  )
  return found ? found.definition : null
}
