# Better Research

Claudine leverages "fleet research" to update it's codebase. This update cycle is necessary because Claudine wraps many underlying agentic platforms that are very actively changing and to use traditional means to stay on top of the high rate of external dependencies would be nearly impossible. At a high level this cycle today looks like the following:

1. Decompose the functional footprint of agentic CLI's and create a research folder for each functional area

    The @claudine/docs/research/README.md has been created to describe each of the research folder's focus in this process.

2. Create either a Claudine **sequence** or **compose** based prompt for each research folder who's responsibility is:

    At the folder level the sequence or prompt is _not_ aware of the
