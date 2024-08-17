Session Manager Connect
=======================

Session Manager Connect is a TUI to simplify using AWS Systems Manager's Session Manager to connect to EC2 instances

# Why?
If you have a more than a few of EC2 servers, you usually do two things:
1. Go to the AWS console, and get the ip address to connect to the server
2. SSH in, usually this means connecting to a VPN, which is annoying to maintain

What if you could save you the VPN, and the looking up the instance information?
This is what this tool does for you.  
It leverages AWS Session Manager to connect to your EC2 instances, which doesn't require you having network connectivity to the instance.
Also, it removes the complexity of connecting to it, by providing an easy way to find which server you want to connect to, and piping out the the correct AWS CLI command.  
Bonus points for not needing SSH anymore. 

# Install
TODO: setup github actions to build binaries for different platforms

Make sure both the AWS CLI and the session manager plugin is installed  
Make sure the your instances are configured to allow connections from Session Manager

# How to use
1. Make sure you are authed to AWS in the terminal.
2. Run sm_connect
3. Select the region where the instance you want to connect is
4. Select the instance you want to connect
5. Enjoy!