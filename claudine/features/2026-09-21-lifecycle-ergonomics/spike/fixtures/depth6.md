---
success:
    - when: "ctx.level == 6"
      then:
          - when: "ctx.level == 5"
            then:
                - when: "ctx.level == 4"
                  then:
                      - when: "ctx.level == 3"
                        then:
                            - when: "ctx.level == 2"
                              then:
                                  - when: "ctx.level == 1"
                                    then:
                                        - say: "leaf"
                                    else:
                                        - warn: "level 1 false"
                              else:
                                  - warn: "level 2 false"
                        else:
                            - warn: "level 3 false"
                  else:
                      - warn: "level 4 false"
            else:
                - warn: "level 5 false"
      else:
          - warn: "level 6 false"
---
# depth 6
