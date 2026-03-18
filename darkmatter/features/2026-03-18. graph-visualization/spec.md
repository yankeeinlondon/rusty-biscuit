# Graph Visualization

In the Darkmatter `compose` functionality we see that a base file we're composing is really the beginning of a graph of files which will be leveraged to produce the output. The graph is influenced by:

- directives like `::file <file-ref>` use transclusion to bring in the content from another file
    - The file which is transcluded can in turn also use the `::file <file-ref>` directive
    - This alone provides a graph structure
- Links
    - local image links are another obvious aspect to the graph:
        - A local file reference like `![image](./image.png)` represents a dependency
        - It's true that when we run **compose** on a Markdown file with images we do not bring in that image to the composed output but for the graph to be valid that image file must exist!
    - remote image links also are similar in concept to local image links, except:
        - The existence of the external image 
