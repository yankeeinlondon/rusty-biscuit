## Best Practices

Please keep in mind the following principles:

- if we capture metadata using the `schematic/define` primitives then the resulting API client generated to `schematic/schema` should ALWAYS have all attributes of that definition; they may be represented differently but there should be no information loss.
    - one area that is easy to overlook is "metadata" attributes like a URL that references something important or a descriptive property that describes something.
    - The core functionality we're delivering may not require these kinds of properties but the goal for the API client we're developing through generation is meant to be self-describing and a user of this client should benefit from ALL metadata that could be useful without being overly verbose
- when planning or executing a plan, if you notice inconsistencies or limitations to design approach -- regardless of whether they are directly a part of your scope of work or not -- they should be documented and reported back.
