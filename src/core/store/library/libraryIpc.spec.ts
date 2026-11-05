// @vitest-environment node
import { describe, expect, it } from 'vitest';
import capabilitiesJson from '../../../../src-tauri/capabilities/default.json?raw';
import permissionDefinitions from '../../../../src-tauri/permissions/main.toml?raw';
import commandRegistration from '../../../../src-tauri/src/lib.rs?raw';

describe('managed library IPC availability', () => {
    it('registers the command and grants it to the main window capability', () => {
        const capability = JSON.parse(capabilitiesJson) as {
            permissions: unknown[];
        };
        const permissions = permissionDefinitions;
        const command = permissions
            .split('[[permission]]')
            .find(block => block.includes('identifier = "allow-library-operation"'));
        expect(command).toContain('commands.allow = ["library_operation"]');
        expect(capability.permissions).toContain('allow-library-operation');
        expect(commandRegistration).toContain(
            'delivery::tauri::commands::library::library_operation,'
        );
        expect(capability.permissions).not.toContain('allow-rename-file');
        expect(capability.permissions).toContain('allow-folder-recovery-directory');
        expect(permissions).toContain('commands.allow = ["folder_recovery_directory"]');
        expect(commandRegistration).toContain(
            'delivery::tauri::commands::library::folder_recovery_directory,'
        );
    });
});
