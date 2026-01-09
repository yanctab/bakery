# Todo

1. ~~Update bkry.bashrc so that it is not tampering with the shell more then needed. Currently doing to much when sourced which can overwrite the users default rc-file that is loaded before.~~
2. See if we should add BKRY_VARIANT and BKRY_HWREVISION as a context variable, --variant, --hwrevision. These variables should then be defined in the local.conf to make it possible to take action in the recipes. The BKRY_VARIANT could replace BKRY_RELEASE but will extend the number of variants
release, dev, test for example.
3. Add support for a hidden .workspace.json.
4. ~~Extend our list of constants.~~
5. Should we update the default workspace configs path to look at /etc instead of looking for local paths in the workspace dir?
6. Make sure all commands are bootstrapped into docker and can run using --interactive flag.
7. Add --env flag to inject variables inside docker.
8. ~~Add --force to setup to allow running setup a second time even when it is non-empty workspace.~~
9. ~~Add --locked to cargo commands.~~
10. ~~Make sure apt is not prompting in the setup-rust script.~~
11. Add shflags to the scripts so it is available for any of the commands and tasks if needed this will make it more structured to follow a convention writing the scripts.
12. Add meta-data support for the workspace. When running setup we should store data for that workspace under the home dir. We should include a workspace ID to easily track a workspace based on an ID.
13. Add --verbose flag and make sure that we get usefull debug information printed.
14. Add support for locking workspace. When running bkry setup the workspace should be locked to the config. We will have to also add a new sub-command bakery lock so it is possible to lock a workspace to a different config and we should be able to reset the build config also. This will need the meta-data support and workspace ID to work. We need to make sure that it is possible to call bkry build without a build config but if there is no workspace meta-data then it should be required.
15. Add support for user specific workspace.json. This will require that our merging of the workspace.json is fully working.
