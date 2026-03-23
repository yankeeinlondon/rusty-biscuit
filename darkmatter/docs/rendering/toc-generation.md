# Table of Contents Generation

It is not uncommon for people to want a **Table of Contents** to be generated for a Markdown file. However, because in a Markdown pipeline we the structure of a document can change during the course of the pipeline, we must put the TOC Generation operation into the final stage of the pipeline (Stage 3: Rendering). This is contrast to [TOC Linking](../preparation/toc-linking.md) which can be done during Stage 1 because it's reporting not on base document but an external document (which has already completed it's own pipeline).

--- 

[< back to **Pipeline Documentation**](../darkmatter-compose-pipeline.md)
