---
name: create-integration
argument-hint: <name> <prompt>
description: |-
    Creates an Unfolded Circle Integration. The Integration will support both local and remote architectures. 
    
    - Local architectures are installed directly onto the remote, versus 
    - Remote architectures have the remote connect over the network to the Integrations socket server
        - this requires that a socket server is up to receive the connection
--- 

**IMPORTANT:** You should use the 'unfolded-circle' skill while executing this command.

The user's requested action is $ARGUMENTS

If the above is empty or says "$ARGUMENTS", stop immediately and reply with:

> You are expected to provide both:
>
> 1. the name of the device or service you are providing an integration for
> 2. a prompt which provides context on the device/service; ideally a design document is referenced
>
> - `/create-integration sony-receiver use the @homelab/docs/`
