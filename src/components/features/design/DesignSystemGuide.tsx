import { Component, createSignal, onMount } from 'solid-js';
import { Button } from '../../ui/Button';
import { RadioGroup, RadioGroupItem } from '../../ui/RadioGroup';
import { Input } from '../../ui/Input';
import { Badge } from '../../ui/Badge';
import { Switch } from '../../ui/Switch';
import { Slider } from '../../ui/Slider';
import { Alert } from '../../ui/Alert';
import { Select } from '../../ui/Select';
import { Checkbox } from '../../ui/Checkbox';
import { ToggleGroup, ToggleGroupItem } from '../../ui/ToggleGroup';
import { ProgressBar } from '../../ui/ProgressBar';
import { Tooltip } from '../../ui/Tooltip';
import { Popover } from '../../ui/Popover';
import { Modal, ConfirmModal } from '../../ui/Modal';
import { PromptModal } from '../../ui/PromptModal';
import { Sonner, toast } from '../../ui/Sonner';
import type { ToasterProps } from '../../ui/Sonner';
import { Kbd } from '../../ui/Kbd';
import { TagInput } from '../../ui/TagInput';
import {
    Search,
    Info,
    Bold,
    Italic,
    Sun,
    Moon,
    AlignCenter,
    AlignLeft,
    AlignRight
} from 'lucide-solid';
import '../../ui/sonner.css'; // Ensure sonner styles are loaded
import { Toggle } from '../../ui';
import { NumberInput } from '../../ui/NumberInput';
import { ColorInput } from '../../ui/ColorInput';
import { DateInput } from '../../ui/DateInput';
import { MaskedInput } from '../../ui/MaskedInput';
import { appearance, appearanceActions } from '../../../core/store/appearanceStore';

export const DesignSystemGuide: Component = () => {
    const [modalSmOpen, setModalSmOpen] = createSignal(false);
    const [modalMdOpen, setModalMdOpen] = createSignal(false);
    const [modalLgOpen, setModalLgOpen] = createSignal(false);
    const [modalXlOpen, setModalXlOpen] = createSignal(false);
    const [modalFullOpen, setModalFullOpen] = createSignal(false);

    const [confirmDangerOpen, setConfirmDangerOpen] = createSignal(false);
    const [confirmWarningOpen, setConfirmWarningOpen] = createSignal(false);
    const [promptOpen, setPromptOpen] = createSignal(false);
    const [sliderVal, setSliderVal] = createSignal(50);
    const [tags, setTags] = createSignal<string[]>(['Design', 'System']);
    const [toastPos, setToastPos] = createSignal<ToasterProps['position']>('bottom-right');

    onMount(() => {
        appearanceActions.initialize();
        window.dispatchEvent(new CustomEvent('app-ready'));
    });

    return (
        <div
            style={{
                'background-color': 'var(--bg-page)',
                color: 'var(--text-primary)',
                'min-height': '100vh',
                padding: 'var(--p-space-xl)',
                'font-family': 'var(--p-font-main)',
                'overflow-y': 'auto',
                height: '100vh',
                'box-sizing': 'border-box'
            }}
        >
            <div style={{ 'max-width': '1200px', margin: '0 auto' }}>
                <header
                    style={{
                        'margin-bottom': 'var(--p-space-2xl)',
                        'border-bottom': '1px solid var(--border-default)',
                        'padding-bottom': 'var(--p-space-l)',
                        display: 'flex',
                        'justify-content': 'space-between',
                        'align-items': 'flex-end'
                    }}
                >
                    <div>
                        <h1
                            style={{
                                'font-size': 'var(--p-font-size-3xl)',
                                'margin-bottom': 'var(--p-space-s)'
                            }}
                        >
                            Elleven Design System
                        </h1>
                        <p style={{ color: 'var(--text-secondary)' }}>
                            Component Guide & Style Reference
                        </p>
                    </div>
                    <div
                        style={{
                            'text-align': 'right',
                            'font-family': 'var(--p-font-mono)',
                            'font-size': 'var(--p-font-size-xxs)',
                            color: 'var(--text-tertiary)',
                            background: 'var(--bg-surface-2)',
                            padding: 'var(--p-space-s) var(--p-space-m)',
                            'border-radius': 'var(--radius-m)',
                            border: '1px solid var(--border-subtle)'
                        }}
                    >
                        <div
                            style={{
                                'font-weight': '600',
                                color: 'var(--action-primary-bg)',
                                'margin-bottom': '4px'
                            }}
                        >
                            ACTIVE SETTINGS
                        </div>
                        <div>Mode: {appearance().mode}</div>
                        <div>Theme: {appearance().theme}</div>
                        <div>Radius: {appearance().radius}px</div>
                        <div>Font: {appearance().fontSize}</div>
                    </div>
                </header>

                <Sonner position={toastPos()} />

                {/* Typography & Colors Section */}
                <section style={{ 'margin-bottom': 'var(--p-space-3xl)' }}>
                    <h2
                        style={{
                            'margin-bottom': 'var(--p-space-l)',
                            'border-bottom': '1px solid var(--border-subtle)',
                            'padding-bottom': 'var(--p-space-s)'
                        }}
                    >
                        1. Tokens & Live Preview
                    </h2>

                    <div
                        style={{
                            display: 'grid',
                            'grid-template-columns': '1fr 1fr',
                            gap: 'var(--p-space-2xl)'
                        }}
                    >
                        <div>
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Color Palette (Current Theme: {appearance().theme})
                            </h3>
                            <div
                                style={{
                                    display: 'flex',
                                    'flex-wrap': 'wrap',
                                    gap: 'var(--p-space-m)'
                                }}
                            >
                                <ColorSwatch name="Primary (500)" val="var(--p-primary-500)" />
                                <ColorSwatch name="Success" val="var(--p-success-500)" />
                                <ColorSwatch name="Warning" val="var(--p-warning-500)" />
                                <ColorSwatch name="Error" val="var(--p-error-500)" />
                                <ColorSwatch name="Info" val="var(--p-info-500)" />
                            </div>
                            <div
                                style={{
                                    display: 'flex',
                                    'flex-wrap': 'wrap',
                                    gap: 'var(--p-space-m)',
                                    'margin-top': 'var(--p-space-m)'
                                }}
                            >
                                <ColorSwatch name="Bg Page" val="var(--bg-page)" border />
                                <ColorSwatch name="Surface 1" val="var(--bg-surface-1)" border />
                                <ColorSwatch name="Surface 2" val="var(--bg-surface-2)" border />
                                <ColorSwatch name="Surface 3" val="var(--bg-surface-3)" border />
                            </div>
                        </div>
                        <div>
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Typography
                            </h3>
                            <div>
                                <p
                                    style={{
                                        'font-size': 'var(--p-font-size-xxs)',
                                        'font-weight': 'var(--p-font-weight-normal)',
                                        'line-height': 'var(--p-line-height-xxs)'
                                    }}
                                >
                                    Typography (XXS)
                                </p>
                                <p
                                    style={{
                                        'font-size': 'var(--p-font-size-xs)',
                                        'font-weight': 'var(--p-font-weight-normal)',
                                        'line-height': 'var(--p-line-height-xs)'
                                    }}
                                >
                                    Typography (XS)
                                </p>
                                <p
                                    style={{
                                        'font-size': 'var(--p-font-size-s)',
                                        'font-weight': 'var(--p-font-weight-normal)',
                                        'line-height': 'var(--p-line-height-s)'
                                    }}
                                >
                                    Typography (S)
                                </p>
                                <p
                                    style={{
                                        'font-size': 'var(--p-font-size-m)',
                                        'font-weight': 'var(--p-font-weight-normal)',
                                        'line-height': 'var(--p-line-height-m)'
                                    }}
                                >
                                    Typography (M)
                                </p>
                                <p
                                    style={{
                                        'font-size': 'var(--p-font-size-l)',
                                        'font-weight': 'var(--p-font-weight-normal)',
                                        'line-height': 'var(--p-line-height-l)'
                                    }}
                                >
                                    Typography (L)
                                </p>
                                <p
                                    style={{
                                        'font-size': 'var(--p-font-size-xl)',
                                        'font-weight': 'var(--p-font-weight-normal)',
                                        'line-height': 'var(--p-line-height-xl)'
                                    }}
                                >
                                    Typography (XL)
                                </p>
                                <p
                                    style={{
                                        'font-size': 'var(--p-font-size-2xl)',
                                        'font-weight': 'var(--p-font-weight-normal)',
                                        'line-height': 'var(--p-line-height-2xl)'
                                    }}
                                >
                                    Typography (2XL)
                                </p>
                                <p
                                    style={{
                                        'font-size': 'var(--p-font-size-3xl)',
                                        'font-weight': 'var(--p-font-weight-normal)',
                                        'line-height': 'var(--p-line-height-3xl)'
                                    }}
                                >
                                    Typography (3XL)
                                </p>
                            </div>
                        </div>
                    </div>
                </section>

                {/* Buttons */}
                <section style={{ 'margin-bottom': 'var(--p-space-3xl)' }}>
                    <h2
                        style={{
                            'margin-bottom': 'var(--p-space-l)',
                            'border-bottom': '1px solid var(--border-subtle)',
                            'padding-bottom': 'var(--p-space-s)'
                        }}
                    >
                        2. Buttons
                    </h2>
                    <div
                        style={{
                            display: 'flex',
                            'flex-direction': 'column',
                            gap: 'var(--p-space-l)'
                        }}
                    >
                        {/* Primary */}
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="primary" size="lg">
                                Primary
                            </Button>
                            <Button variant="primary" size="lg" disabled>
                                Primary
                            </Button>
                            <Button variant="primary" size="lg" loading>
                                Loading
                            </Button>
                            <Button variant="primary" size="lg" leftIcon={<Search />}>
                                Primary
                            </Button>
                            <Button variant="primary" size="lg" rightIcon={<Search />}>
                                Primary
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="primary" size="md">
                                Primary
                            </Button>
                            <Button variant="primary" size="md" disabled>
                                Primary
                            </Button>
                            <Button variant="primary" size="md" loading>
                                Loading
                            </Button>
                            <Button variant="primary" size="md" leftIcon={<Search />}>
                                Primary
                            </Button>
                            <Button variant="primary" size="md" rightIcon={<Search />}>
                                Primary
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="primary" size="sm">
                                Primary
                            </Button>
                            <Button variant="primary" size="sm" disabled>
                                Primary
                            </Button>
                            <Button variant="primary" size="sm" loading>
                                Loading
                            </Button>
                            <Button variant="primary" size="sm" leftIcon={<Search />}>
                                Primary
                            </Button>
                            <Button variant="primary" size="sm" rightIcon={<Search />}>
                                Primary
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="primary" size="xs">
                                Primary
                            </Button>
                            <Button variant="primary" size="xs" disabled>
                                Primary
                            </Button>
                            <Button variant="primary" size="xs" loading>
                                Loading
                            </Button>
                            <Button variant="primary" size="xs" leftIcon={<Search />}>
                                Primary
                            </Button>
                            <Button variant="primary" size="xs" rightIcon={<Search />}>
                                Primary
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="primary" size="icon">
                                <Search />
                            </Button>
                            <Button variant="primary" size="icon" disabled>
                                <Search />
                            </Button>
                            <Button variant="primary" size="icon-sm">
                                <Search />
                            </Button>
                            <Button variant="primary" size="icon-sm" disabled>
                                <Search />
                            </Button>
                            <Button variant="primary" size="icon-xs">
                                <Search />
                            </Button>
                            <Button variant="primary" size="icon-xs" disabled>
                                <Search />
                            </Button>
                        </div>

                        {/* Secondary */}
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="secondary" size="lg">
                                Secondary
                            </Button>
                            <Button variant="secondary" size="lg" disabled>
                                Secondary
                            </Button>
                            <Button variant="secondary" size="lg" loading>
                                Loading
                            </Button>
                            <Button variant="secondary" size="lg" leftIcon={<Search />}>
                                Secondary
                            </Button>
                            <Button variant="secondary" size="lg" rightIcon={<Search />}>
                                Secondary
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="secondary" size="md">
                                Secondary
                            </Button>
                            <Button variant="secondary" size="md" disabled>
                                Secondary
                            </Button>
                            <Button variant="secondary" size="md" loading>
                                Loading
                            </Button>
                            <Button variant="secondary" size="md" leftIcon={<Search />}>
                                Secondary
                            </Button>
                            <Button variant="secondary" size="md" rightIcon={<Search />}>
                                Secondary
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="secondary" size="sm">
                                Secondary
                            </Button>
                            <Button variant="secondary" size="sm" disabled>
                                Secondary
                            </Button>
                            <Button variant="secondary" size="sm" loading>
                                Loading
                            </Button>
                            <Button variant="secondary" size="sm" leftIcon={<Search />}>
                                Secondary
                            </Button>
                            <Button variant="secondary" size="sm" rightIcon={<Search />}>
                                Secondary
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="secondary" size="xs">
                                Secondary
                            </Button>
                            <Button variant="secondary" size="xs" disabled>
                                Secondary
                            </Button>
                            <Button variant="secondary" size="xs" loading>
                                Loading
                            </Button>
                            <Button variant="secondary" size="xs" leftIcon={<Search />}>
                                Secondary
                            </Button>
                            <Button variant="secondary" size="xs" rightIcon={<Search />}>
                                Secondary
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="secondary" size="icon">
                                <Search />
                            </Button>
                            <Button variant="secondary" size="icon" disabled>
                                <Search />
                            </Button>
                            <Button variant="secondary" size="icon-sm">
                                <Search />
                            </Button>
                            <Button variant="secondary" size="icon-sm" disabled>
                                <Search />
                            </Button>
                            <Button variant="secondary" size="icon-xs">
                                <Search />
                            </Button>
                            <Button variant="secondary" size="icon-xs" disabled>
                                <Search />
                            </Button>
                        </div>

                        {/* Outline */}
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="outline" size="lg">
                                Outline
                            </Button>
                            <Button variant="outline" size="lg" disabled>
                                Outline
                            </Button>
                            <Button variant="outline" size="lg" loading>
                                Loading
                            </Button>
                            <Button variant="outline" size="lg" leftIcon={<Search />}>
                                Outline
                            </Button>
                            <Button variant="outline" size="lg" rightIcon={<Search />}>
                                Outline
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="outline" size="md">
                                Outline
                            </Button>
                            <Button variant="outline" size="md" disabled>
                                Outline
                            </Button>
                            <Button variant="outline" size="md" loading>
                                Loading
                            </Button>
                            <Button variant="outline" size="md" leftIcon={<Search />}>
                                Outline
                            </Button>
                            <Button variant="outline" size="md" rightIcon={<Search />}>
                                Outline
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="outline" size="sm">
                                Outline
                            </Button>
                            <Button variant="outline" size="sm" disabled>
                                Outline
                            </Button>
                            <Button variant="outline" size="sm" loading>
                                Loading
                            </Button>
                            <Button variant="outline" size="sm" leftIcon={<Search />}>
                                Outline
                            </Button>
                            <Button variant="outline" size="sm" rightIcon={<Search />}>
                                Outline
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="outline" size="xs">
                                Outline
                            </Button>
                            <Button variant="outline" size="xs" disabled>
                                Outline
                            </Button>
                            <Button variant="outline" size="xs" loading>
                                Loading
                            </Button>
                            <Button variant="outline" size="xs" leftIcon={<Search />}>
                                Outline
                            </Button>
                            <Button variant="outline" size="xs" rightIcon={<Search />}>
                                Outline
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="outline" size="icon">
                                <Search />
                            </Button>
                            <Button variant="outline" size="icon" disabled>
                                <Search />
                            </Button>
                            <Button variant="outline" size="icon-sm">
                                <Search />
                            </Button>
                            <Button variant="outline" size="icon-sm" disabled>
                                <Search />
                            </Button>
                            <Button variant="outline" size="icon-xs">
                                <Search />
                            </Button>
                            <Button variant="outline" size="icon-xs" disabled>
                                <Search />
                            </Button>
                        </div>

                        {/* Ghost */}
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost" size="lg">
                                Ghost
                            </Button>
                            <Button variant="ghost" size="lg" disabled>
                                Ghost
                            </Button>
                            <Button variant="ghost" size="lg" loading>
                                Loading
                            </Button>
                            <Button variant="ghost" size="lg" leftIcon={<Search />}>
                                Ghost
                            </Button>
                            <Button variant="ghost" size="lg" rightIcon={<Search />}>
                                Ghost
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost" size="md">
                                Ghost
                            </Button>
                            <Button variant="ghost" size="md" disabled>
                                Ghost
                            </Button>
                            <Button variant="ghost" size="md" loading>
                                Loading
                            </Button>
                            <Button variant="ghost" size="md" leftIcon={<Search />}>
                                Ghost
                            </Button>
                            <Button variant="ghost" size="md" rightIcon={<Search />}>
                                Ghost
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost" size="sm">
                                Ghost
                            </Button>
                            <Button variant="ghost" size="sm" disabled>
                                Ghost
                            </Button>
                            <Button variant="ghost" size="sm" loading>
                                Loading
                            </Button>
                            <Button variant="ghost" size="sm" leftIcon={<Search />}>
                                Ghost
                            </Button>
                            <Button variant="ghost" size="sm" rightIcon={<Search />}>
                                Ghost
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost" size="xs">
                                Ghost
                            </Button>
                            <Button variant="ghost" size="xs" disabled>
                                Ghost
                            </Button>
                            <Button variant="ghost" size="xs" loading>
                                Loading
                            </Button>
                            <Button variant="ghost" size="xs" leftIcon={<Search />}>
                                Ghost
                            </Button>
                            <Button variant="ghost" size="xs" rightIcon={<Search />}>
                                Ghost
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost" size="icon">
                                <Search />
                            </Button>
                            <Button variant="ghost" size="icon" disabled>
                                <Search />
                            </Button>
                            <Button variant="ghost" size="icon-sm">
                                <Search />
                            </Button>
                            <Button variant="ghost" size="icon-sm" disabled>
                                <Search />
                            </Button>
                            <Button variant="ghost" size="icon-xs">
                                <Search />
                            </Button>
                            <Button variant="ghost" size="icon-xs" disabled>
                                <Search />
                            </Button>
                        </div>

                        {/* Ghost Destructive */}
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost-destructive" size="lg">
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="lg" disabled>
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="lg" loading>
                                Loading
                            </Button>
                            <Button variant="ghost-destructive" size="lg" leftIcon={<Search />}>
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="lg" rightIcon={<Search />}>
                                Ghost Destructive
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost-destructive" size="md">
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="md" disabled>
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="md" loading>
                                Loading
                            </Button>
                            <Button variant="ghost-destructive" size="md" leftIcon={<Search />}>
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="md" rightIcon={<Search />}>
                                Ghost Destructive
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost-destructive" size="sm">
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="sm" disabled>
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="sm" loading>
                                Loading
                            </Button>
                            <Button variant="ghost-destructive" size="sm" leftIcon={<Search />}>
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="sm" rightIcon={<Search />}>
                                Ghost Destructive
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost-destructive" size="xs">
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="xs" disabled>
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="xs" loading>
                                Loading
                            </Button>
                            <Button variant="ghost-destructive" size="xs" leftIcon={<Search />}>
                                Ghost Destructive
                            </Button>
                            <Button variant="ghost-destructive" size="xs" rightIcon={<Search />}>
                                Ghost Destructive
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="ghost-destructive" size="icon">
                                <Search />
                            </Button>
                            <Button variant="ghost-destructive" size="icon" disabled>
                                <Search />
                            </Button>
                            <Button variant="ghost-destructive" size="icon-sm">
                                <Search />
                            </Button>
                            <Button variant="ghost-destructive" size="icon-sm" disabled>
                                <Search />
                            </Button>
                            <Button variant="ghost-destructive" size="icon-xs">
                                <Search />
                            </Button>
                            <Button variant="ghost-destructive" size="icon-xs" disabled>
                                <Search />
                            </Button>
                        </div>

                        {/* Destructive */}
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="destructive" size="lg">
                                Destructive
                            </Button>
                            <Button variant="destructive" size="lg" disabled>
                                Destructive
                            </Button>
                            <Button variant="destructive" size="lg" loading>
                                Loading
                            </Button>
                            <Button variant="destructive" size="lg" leftIcon={<Search />}>
                                Destructive
                            </Button>
                            <Button variant="destructive" size="lg" rightIcon={<Search />}>
                                Destructive
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="destructive" size="md">
                                Destructive
                            </Button>
                            <Button variant="destructive" size="md" disabled>
                                Destructive
                            </Button>
                            <Button variant="destructive" size="md" loading>
                                Loading
                            </Button>
                            <Button variant="destructive" size="md" leftIcon={<Search />}>
                                Destructive
                            </Button>
                            <Button variant="destructive" size="md" rightIcon={<Search />}>
                                Destructive
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="destructive" size="sm">
                                Destructive
                            </Button>
                            <Button variant="destructive" size="sm" disabled>
                                Destructive
                            </Button>
                            <Button variant="destructive" size="sm" loading>
                                Loading
                            </Button>
                            <Button variant="destructive" size="sm" leftIcon={<Search />}>
                                Destructive
                            </Button>
                            <Button variant="destructive" size="sm" rightIcon={<Search />}>
                                Destructive
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="destructive" size="xs">
                                Destructive
                            </Button>
                            <Button variant="destructive" size="xs" disabled>
                                Destructive
                            </Button>
                            <Button variant="destructive" size="xs" loading>
                                Loading
                            </Button>
                            <Button variant="destructive" size="xs" leftIcon={<Search />}>
                                Destructive
                            </Button>
                            <Button variant="destructive" size="xs" rightIcon={<Search />}>
                                Destructive
                            </Button>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Button variant="destructive" size="icon">
                                <Search />
                            </Button>
                            <Button variant="destructive" size="icon" disabled>
                                <Search />
                            </Button>
                            <Button variant="destructive" size="icon-sm">
                                <Search />
                            </Button>
                            <Button variant="destructive" size="icon-sm" disabled>
                                <Search />
                            </Button>
                            <Button variant="destructive" size="icon-xs">
                                <Search />
                            </Button>
                            <Button variant="destructive" size="icon-xs" disabled>
                                <Search />
                            </Button>
                        </div>
                    </div>
                </section>

                {/* Toggle */}
                <section style={{ 'margin-bottom': 'var(--p-space-3xl)' }}>
                    <h2
                        style={{
                            'margin-bottom': 'var(--p-space-l)',
                            'border-bottom': '1px solid var(--border-subtle)',
                            'padding-bottom': 'var(--p-space-s)'
                        }}
                    >
                        3. Toggle
                    </h2>
                    <div
                        style={{
                            display: 'flex',
                            'flex-direction': 'column',
                            gap: 'var(--p-space-l)'
                        }}
                    >
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Toggle aria-label="Toggle" pressed={true} size="lg">
                                <Sun /> teste
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} size="lg">
                                <Moon />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} size="lg" disabled>
                                <Moon />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} variant="outline" size="lg">
                                <Sun />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} variant="outline" size="lg">
                                <Moon />
                            </Toggle>
                            <Toggle
                                aria-label="Toggle"
                                pressed={false}
                                variant="outline"
                                size="lg"
                                disabled
                            >
                                <Moon />
                            </Toggle>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Toggle aria-label="Toggle" pressed={true} size="md">
                                <Sun /> teste
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} size="md">
                                <Moon />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} size="md" disabled>
                                <Moon />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} variant="outline" size="md">
                                <Sun />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} variant="outline" size="md">
                                <Moon />
                            </Toggle>
                            <Toggle
                                aria-label="Toggle"
                                pressed={false}
                                variant="outline"
                                size="md"
                                disabled
                            >
                                <Moon />
                            </Toggle>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            <Toggle aria-label="Toggle" pressed={true} size="sm">
                                <Sun /> teste
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} size="sm">
                                <Moon />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} size="sm" disabled>
                                <Moon />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} variant="outline" size="sm">
                                <Sun />
                            </Toggle>
                            <Toggle aria-label="Toggle" pressed={false} variant="outline" size="sm">
                                <Moon />
                            </Toggle>
                            <Toggle
                                aria-label="Toggle"
                                pressed={false}
                                variant="outline"
                                size="sm"
                                disabled
                            >
                                <Moon />
                            </Toggle>
                        </div>
                    </div>
                </section>

                {/* Toggle group */}
                <section style={{ 'margin-bottom': 'var(--p-space-3xl)' }}>
                    <h2
                        style={{
                            'margin-bottom': 'var(--p-space-l)',
                            'border-bottom': '1px solid var(--border-subtle)',
                            'padding-bottom': 'var(--p-space-s)'
                        }}
                    >
                        4. Toggle Group
                    </h2>
                    <div
                        style={{
                            display: 'flex',
                            'flex-direction': 'column',
                            gap: 'var(--p-space-l)'
                        }}
                    >
                        {/* size: xl */}
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            {/* Single selection xl */}
                            <ToggleGroup
                                type="single"
                                orientation="horizontal"
                                defaultValue="center"
                                size="xl"
                            >
                                <ToggleGroupItem value="left">
                                    <AlignLeft />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="center">
                                    <AlignCenter />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="right" disabled>
                                    <AlignRight />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Multiple selection xl */}
                            <ToggleGroup
                                type="multiple"
                                orientation="horizontal"
                                defaultValue={['bold']}
                                size="xl"
                            >
                                <ToggleGroupItem value="bold">
                                    <Bold />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="italic">
                                    <Italic />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Single selection lg */}
                            <ToggleGroup
                                type="single"
                                orientation="horizontal"
                                defaultValue="center"
                                size="lg"
                            >
                                <ToggleGroupItem value="left">
                                    <AlignLeft />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="center">
                                    <AlignCenter />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="right" disabled>
                                    <AlignRight />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Multiple selection lg */}
                            <ToggleGroup
                                type="multiple"
                                orientation="horizontal"
                                defaultValue={['bold']}
                                size="lg"
                            >
                                <ToggleGroupItem value="bold">
                                    <Bold />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="italic">
                                    <Italic />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Single selection md */}
                            <ToggleGroup
                                type="single"
                                orientation="horizontal"
                                defaultValue="center"
                                size="md"
                            >
                                <ToggleGroupItem value="left">
                                    <AlignLeft />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="center">
                                    <AlignCenter />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="right" disabled>
                                    <AlignRight />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Multiple selection md */}
                            <ToggleGroup
                                type="multiple"
                                orientation="horizontal"
                                defaultValue={['bold']}
                                size="md"
                            >
                                <ToggleGroupItem value="bold">
                                    <Bold />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="italic">
                                    <Italic />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Single selection sm */}
                            <ToggleGroup
                                type="single"
                                orientation="horizontal"
                                defaultValue="center"
                                size="sm"
                            >
                                <ToggleGroupItem value="left">
                                    <AlignLeft />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="center">
                                    <AlignCenter />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="right" disabled>
                                    <AlignRight />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Multiple selection sm */}
                            <ToggleGroup
                                type="multiple"
                                orientation="horizontal"
                                defaultValue={['bold']}
                                size="sm"
                            >
                                <ToggleGroupItem value="bold">
                                    <Bold />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="italic">
                                    <Italic />
                                </ToggleGroupItem>
                            </ToggleGroup>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                gap: 'var(--p-space-m)',
                                'align-items': 'center'
                            }}
                        >
                            {/* Single selection vertical xl */}
                            <ToggleGroup
                                type="single"
                                orientation="vertical"
                                defaultValue="center"
                                size="xl"
                            >
                                <ToggleGroupItem value="left">
                                    <AlignLeft />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="center">
                                    <AlignCenter />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="right" disabled>
                                    <AlignRight />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Multiple selection vertical xl */}
                            <ToggleGroup
                                type="multiple"
                                orientation="vertical"
                                defaultValue={['bold']}
                                size="xl"
                            >
                                <ToggleGroupItem value="bold">
                                    <Bold />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="italic">
                                    <Italic />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Single selection vertical lg */}
                            <ToggleGroup
                                type="single"
                                orientation="vertical"
                                defaultValue="center"
                                size="lg"
                            >
                                <ToggleGroupItem value="left">
                                    <AlignLeft />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="center">
                                    <AlignCenter />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="right" disabled>
                                    <AlignRight />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Multiple selection vertical lg */}
                            <ToggleGroup
                                type="multiple"
                                orientation="vertical"
                                defaultValue={['bold']}
                                size="lg"
                            >
                                <ToggleGroupItem value="bold">
                                    <Bold />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="italic">
                                    <Italic />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Single selection vertical md */}
                            <ToggleGroup
                                type="single"
                                orientation="vertical"
                                defaultValue="center"
                                size="md"
                            >
                                <ToggleGroupItem value="left">
                                    <AlignLeft />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="center">
                                    <AlignCenter />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="right" disabled>
                                    <AlignRight />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Multiple selection vertical md */}
                            <ToggleGroup
                                type="multiple"
                                orientation="vertical"
                                defaultValue={['bold']}
                                size="md"
                            >
                                <ToggleGroupItem value="bold">
                                    <Bold />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="italic">
                                    <Italic />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Single selection vertical sm */}
                            <ToggleGroup
                                type="single"
                                orientation="vertical"
                                defaultValue="center"
                                size="sm"
                            >
                                <ToggleGroupItem value="left">
                                    <AlignLeft />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="center">
                                    <AlignCenter />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="right" disabled>
                                    <AlignRight />
                                </ToggleGroupItem>
                            </ToggleGroup>

                            {/* Multiple selection vertical sm */}
                            <ToggleGroup
                                type="multiple"
                                orientation="vertical"
                                defaultValue={['bold']}
                                size="sm"
                            >
                                <ToggleGroupItem value="bold">
                                    <Bold />
                                </ToggleGroupItem>
                                <ToggleGroupItem value="italic">
                                    <Italic />
                                </ToggleGroupItem>
                            </ToggleGroup>
                        </div>
                    </div>
                </section>

                {/* Form Elements */}
                <section style={{ 'margin-bottom': 'var(--p-space-3xl)' }}>
                    <h2
                        style={{
                            'margin-bottom': 'var(--p-space-l)',
                            'border-bottom': '1px solid var(--border-subtle)',
                            'padding-bottom': 'var(--p-space-s)'
                        }}
                    >
                        3. Form Elements
                    </h2>
                    <div
                        style={{
                            display: 'grid',
                            'grid-template-columns': 'repeat(3, 1fr)',
                            gap: 'var(--p-space-xl)'
                        }}
                    >
                        <div
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: 'var(--p-space-m)'
                            }}
                        >
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Inputs with left icons
                            </h3>
                            <Input
                                placeholder="Standard Input..."
                                size="lg"
                                leftIcon={<Search />}
                            />
                            <Input
                                placeholder="Standard Input..."
                                size="md"
                                leftIcon={<Search />}
                            />
                            <Input
                                placeholder="Standard Input..."
                                size="sm"
                                leftIcon={<Search />}
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Number Inputs
                            </h3>
                            <NumberInput placeholder="Number (lg)" size="lg" min={0} max={100} />
                            <NumberInput
                                placeholder="Number (md)"
                                size="md"
                                defaultValue={10}
                                step={5}
                            />
                            <NumberInput placeholder="Number (sm)" size="sm" />
                            <NumberInput
                                placeholder="Disabled (lg)"
                                size="lg"
                                disabled
                                value={50}
                            />
                            <NumberInput
                                placeholder="Disabled (md)"
                                size="md"
                                disabled
                                value={50}
                            />
                            <NumberInput
                                placeholder="Disabled (sm)"
                                size="sm"
                                disabled
                                value={50}
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Selects with left icons
                            </h3>
                            <Select
                                options={[
                                    { label: 'Item 1', value: '1' },
                                    { label: 'Item 2', value: '2' },
                                    { label: 'Item 3', value: '3' }
                                ]}
                                placeholder="Select an item (lg)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="lg"
                                leftIcon={<Search />}
                            />
                            <Select
                                options={[
                                    { label: 'Item 1', value: '1' },
                                    { label: 'Item 2', value: '2' },
                                    { label: 'Item 3', value: '3' }
                                ]}
                                placeholder="Select an item (md)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="md"
                                leftIcon={<Search />}
                            />
                            <Select
                                options={[
                                    { label: 'Item 1', value: '1' },
                                    { label: 'Item 2', value: '2' },
                                    { label: 'Item 3', value: '3' }
                                ]}
                                placeholder="Select an item (sm)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="sm"
                                leftIcon={<Search />}
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Checkboxes
                            </h3>

                            <div
                                style={{
                                    display: 'flex',
                                    gap: 'var(--p-space-m)',
                                    'align-items': 'center'
                                }}
                            >
                                <div
                                    style={{
                                        display: 'grid',
                                        'grid-template-columns': 'repeat(3, 1fr)',
                                        gap: 'var(--p-space-m)'
                                    }}
                                >
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <Checkbox
                                            defaultChecked={true}
                                            label="Checkbox"
                                            size="lg"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={false}
                                            label="Checkbox"
                                            size="lg"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={true}
                                            label="Checkbox"
                                            size="lg"
                                            disabled
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={false}
                                            label="Checkbox"
                                            size="lg"
                                            disabled
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            indeterminate={true}
                                            label="Checkbox"
                                            size="lg"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            indeterminate={true}
                                            label="Checkbox"
                                            size="lg"
                                            disabled
                                            description="Checkbox description"
                                        />
                                    </div>
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <Checkbox
                                            defaultChecked={true}
                                            label="Checkbox"
                                            size="md"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={false}
                                            label="Checkbox"
                                            size="md"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={true}
                                            label="Checkbox"
                                            size="md"
                                            disabled
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={false}
                                            label="Checkbox"
                                            size="md"
                                            disabled
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            indeterminate={true}
                                            label="Checkbox"
                                            size="md"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            indeterminate={true}
                                            label="Checkbox"
                                            size="md"
                                            disabled
                                            description="Checkbox description"
                                        />
                                    </div>
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <Checkbox
                                            defaultChecked={true}
                                            label="Checkbox"
                                            size="sm"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={false}
                                            label="Checkbox"
                                            size="sm"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={true}
                                            label="Checkbox"
                                            size="sm"
                                            disabled
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            defaultChecked={false}
                                            label="Checkbox"
                                            size="sm"
                                            disabled
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            indeterminate={true}
                                            label="Checkbox"
                                            size="sm"
                                            description="Checkbox description"
                                        />
                                        <Checkbox
                                            indeterminate={true}
                                            label="Checkbox"
                                            size="sm"
                                            disabled
                                            description="Checkbox description"
                                        />
                                    </div>
                                </div>
                            </div>

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Switches
                            </h3>

                            <div
                                style={{
                                    display: 'flex',
                                    gap: 'var(--p-space-m)',
                                    'align-items': 'start'
                                }}
                            >
                                <div
                                    style={{
                                        display: 'grid',
                                        'grid-template-columns': 'repeat(3, 1fr)',
                                        gap: 'var(--p-space-m)'
                                    }}
                                >
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <Switch
                                            defaultChecked={true}
                                            label="Switch"
                                            size="lg"
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={false}
                                            label="Switch"
                                            size="lg"
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={true}
                                            label="Switch"
                                            size="lg"
                                            disabled
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={false}
                                            label="Switch"
                                            size="lg"
                                            disabled
                                            description="Switch description"
                                        />
                                    </div>
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <Switch
                                            defaultChecked={true}
                                            label="Switch"
                                            size="md"
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={false}
                                            label="Switch"
                                            size="md"
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={true}
                                            label="Switch"
                                            size="md"
                                            disabled
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={false}
                                            label="Switch"
                                            size="md"
                                            disabled
                                            description="Switch description"
                                        />
                                    </div>
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <Switch
                                            defaultChecked={true}
                                            label="Switch"
                                            size="sm"
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={false}
                                            label="Switch"
                                            size="sm"
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={true}
                                            label="Switch"
                                            size="sm"
                                            disabled
                                            description="Switch description"
                                        />
                                        <Switch
                                            defaultChecked={false}
                                            label="Switch"
                                            size="sm"
                                            disabled
                                            description="Switch description"
                                        />
                                    </div>
                                </div>
                            </div>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: 'var(--p-space-m)'
                            }}
                        >
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Inputs with right icons
                            </h3>
                            <Input
                                placeholder="Standard Input..."
                                size="lg"
                                rightIcon={<Search />}
                            />
                            <Input
                                placeholder="Standard Input..."
                                size="md"
                                rightIcon={<Search />}
                            />
                            <Input
                                placeholder="Standard Input..."
                                size="sm"
                                rightIcon={<Search />}
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Color Inputs
                            </h3>
                            <ColorInput defaultValue="#3b82f6" size="lg" />
                            <ColorInput defaultValue="#22c55e" size="md" />
                            <ColorInput defaultValue="#eab308" size="sm" />
                            <ColorInput defaultValue="#ef4444" size="lg" disabled />
                            <ColorInput defaultValue="#ef4444" size="md" disabled />
                            <ColorInput defaultValue="#ef4444" size="sm" disabled />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Selects with right icons
                            </h3>
                            <Select
                                options={[
                                    { label: 'Item 1', value: '1' },
                                    { label: 'Item 2', value: '2' },
                                    { label: 'Item 3', value: '3' }
                                ]}
                                placeholder="Select an item (lg)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="lg"
                                rightIcon={<Search />}
                            />
                            <Select
                                options={[
                                    { label: 'Item 1', value: '1' },
                                    { label: 'Item 2', value: '2' },
                                    { label: 'Item 3', value: '3' }
                                ]}
                                placeholder="Select an item (md)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="md"
                                rightIcon={<Search />}
                            />
                            <Select
                                options={[
                                    { label: 'Item 1', value: '1' },
                                    { label: 'Item 2', value: '2' },
                                    { label: 'Item 3', value: '3' }
                                ]}
                                placeholder="Select an item (sm)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="sm"
                                rightIcon={<Search />}
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Radio Buttons
                            </h3>

                            <div
                                style={{
                                    display: 'flex',
                                    gap: 'var(--p-space-m)',
                                    'align-items': 'start'
                                }}
                            >
                                <div
                                    style={{
                                        display: 'grid',
                                        'grid-template-columns': 'repeat(3, 1fr)',
                                        gap: 'var(--p-space-m)'
                                    }}
                                >
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <RadioGroup defaultValue="a" name="radio-lg">
                                            <RadioGroupItem
                                                value="a"
                                                label="Radio Option"
                                                size="lg"
                                                description="Radio description"
                                            />
                                            <RadioGroupItem
                                                value="b"
                                                label="Radio Option"
                                                size="lg"
                                                description="Radio description"
                                            />
                                        </RadioGroup>
                                        <RadioGroup
                                            defaultValue="a"
                                            disabled
                                            name="radio-lg-disabled"
                                        >
                                            <RadioGroupItem
                                                value="a"
                                                label="Radio Option"
                                                size="lg"
                                                description="Radio description"
                                            />
                                            <RadioGroupItem
                                                value="b"
                                                label="Radio Option"
                                                size="lg"
                                                description="Radio description"
                                            />
                                        </RadioGroup>
                                    </div>
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <RadioGroup defaultValue="a" name="radio-md">
                                            <RadioGroupItem
                                                value="a"
                                                label="Radio Option"
                                                size="md"
                                                description="Radio description"
                                            />
                                            <RadioGroupItem
                                                value="b"
                                                label="Radio Option"
                                                size="md"
                                                description="Radio description"
                                            />
                                        </RadioGroup>
                                        <RadioGroup
                                            defaultValue="a"
                                            disabled
                                            name="radio-md-disabled"
                                        >
                                            <RadioGroupItem
                                                value="a"
                                                label="Radio Option"
                                                size="md"
                                                description="Radio description"
                                            />
                                            <RadioGroupItem
                                                value="b"
                                                label="Radio Option"
                                                size="md"
                                                description="Radio description"
                                            />
                                        </RadioGroup>
                                    </div>
                                    <div
                                        style={{
                                            display: 'flex',
                                            'flex-direction': 'column',
                                            gap: 'var(--p-space-s)'
                                        }}
                                    >
                                        <RadioGroup defaultValue="a" name="radio-sm">
                                            <RadioGroupItem
                                                value="a"
                                                label="Radio Option"
                                                size="sm"
                                                description="Radio description"
                                            />
                                            <RadioGroupItem
                                                value="b"
                                                label="Radio Option"
                                                size="sm"
                                                description="Radio description"
                                            />
                                        </RadioGroup>
                                        <RadioGroup
                                            defaultValue="a"
                                            disabled
                                            name="radio-sm-disabled"
                                        >
                                            <RadioGroupItem
                                                value="a"
                                                label="Radio Option"
                                                size="sm"
                                                description="Radio description"
                                            />
                                            <RadioGroupItem
                                                value="b"
                                                label="Radio Option"
                                                size="sm"
                                                description="Radio description"
                                            />
                                        </RadioGroup>
                                    </div>
                                </div>
                            </div>

                            <div style={{ 'margin-top': 'var(--p-space-m)' }}>
                                <label
                                    style={{
                                        'margin-bottom': 'var(--p-space-s)',
                                        display: 'block'
                                    }}
                                >
                                    Horizontal Radio Group (lg)
                                </label>
                                <RadioGroup
                                    defaultValue="a"
                                    name="radio-horizontal-lg"
                                    orientation="horizontal"
                                >
                                    <RadioGroupItem value="a" label="Option A" size="lg" />
                                    <RadioGroupItem value="b" label="Option B" size="lg" />
                                    <RadioGroupItem value="c" label="Option C" size="lg" />
                                </RadioGroup>
                            </div>
                            <div style={{ 'margin-top': 'var(--p-space-m)' }}>
                                <label
                                    style={{
                                        'margin-bottom': 'var(--p-space-s)',
                                        display: 'block'
                                    }}
                                >
                                    Horizontal Radio Group (md)
                                </label>
                                <RadioGroup
                                    defaultValue="a"
                                    name="radio-horizontal"
                                    orientation="horizontal"
                                >
                                    <RadioGroupItem value="a" label="Option A" size="md" />
                                    <RadioGroupItem value="b" label="Option B" size="md" />
                                    <RadioGroupItem value="c" label="Option C" size="md" />
                                </RadioGroup>
                            </div>
                            <div style={{ 'margin-top': 'var(--p-space-m)' }}>
                                <label
                                    style={{
                                        'margin-bottom': 'var(--p-space-s)',
                                        display: 'block'
                                    }}
                                >
                                    Horizontal Radio Group (sm)
                                </label>
                                <RadioGroup
                                    defaultValue="a"
                                    name="radio-horizontal-sm"
                                    orientation="horizontal"
                                >
                                    <RadioGroupItem value="a" label="Option A" size="sm" />
                                    <RadioGroupItem value="b" label="Option B" size="sm" />
                                    <RadioGroupItem value="c" label="Option C" size="sm" />
                                </RadioGroup>
                            </div>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: 'var(--p-space-m)'
                            }}
                        >
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Inputs with both icons and disabled
                            </h3>
                            <Input
                                placeholder="Standard Input..."
                                size="lg"
                                leftIcon={<Search />}
                                rightIcon={<Search />}
                                disabled
                            />
                            <Input
                                placeholder="Standard Input..."
                                size="md"
                                leftIcon={<Search />}
                                rightIcon={<Search />}
                                disabled
                            />
                            <Input
                                placeholder="Standard Input..."
                                size="sm"
                                leftIcon={<Search />}
                                rightIcon={<Search />}
                                disabled
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Date Inputs
                            </h3>

                            <DateInput placeholder="DD/MM/YYYY" size="lg" />
                            <DateInput placeholder="DD/MM/YYYY" size="md" />
                            <DateInput placeholder="DD/MM/YYYY" size="sm" />

                            <DateInput placeholder="Disabled" size="lg" disabled />
                            <DateInput placeholder="Disabled" size="md" disabled />
                            <DateInput placeholder="Disabled" size="sm" disabled />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Selects with both icons and disabled
                            </h3>
                            <Select
                                options={[]}
                                placeholder="Select (lg, disabled)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="lg"
                                leftIcon={<Search />}
                                rightIcon={<Search />}
                                disabled
                            />
                            <Select
                                options={[]}
                                placeholder="Select (md, disabled)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="md"
                                leftIcon={<Search />}
                                rightIcon={<Search />}
                                disabled
                            />
                            <Select
                                options={[]}
                                placeholder="Select (sm, disabled)"
                                value={undefined}
                                onValueChange={() => {}}
                                size="sm"
                                leftIcon={<Search />}
                                rightIcon={<Search />}
                                disabled
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Masked Inputs (Phone)
                            </h3>
                            <MaskedInput
                                mask="(99) 99999-9999"
                                placeholder="(99) 99999-9999"
                                size="lg"
                            />
                            <MaskedInput
                                mask="(99) 99999-9999"
                                placeholder="(99) 99999-9999"
                                size="md"
                            />
                            <MaskedInput
                                mask="(99) 99999-9999"
                                placeholder="(99) 99999-9999"
                                size="sm"
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Tag Input
                            </h3>
                            <TagInput
                                value={tags().map((t, i) => ({ id: i, label: t }))}
                                onChange={vals => setTags(vals.map(v => v.label))}
                                onCreate={t => setTags([...tags(), t])}
                                placeholder="Add tags..."
                                suggestions={[
                                    { id: 1, label: 'tag1' },
                                    { id: 2, label: 'tag2' },
                                    { id: 3, label: 'tag3' }
                                ]}
                            />

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Slider
                            </h3>
                            <label>Slider ({Math.round(sliderVal())})</label>
                            <div
                                style={{
                                    display: 'flex',
                                    'flex-direction': 'row',
                                    gap: 'var(--p-space-m)'
                                }}
                            >
                                <div
                                    style={{
                                        display: 'flex',
                                        width: '100%',
                                        'flex-direction': 'column',
                                        gap: 'var(--p-space-m)'
                                    }}
                                >
                                    <Slider
                                        value={sliderVal()}
                                        onValueChange={setSliderVal}
                                        min={0}
                                        max={100}
                                    />
                                    <Slider defaultValue={50} min={0} max={100} disabled />
                                    <Slider defaultValue={50} step={25} min={0} max={100} />
                                    <Slider
                                        defaultValue={50}
                                        step={25}
                                        min={0}
                                        max={100}
                                        disabled
                                    />
                                </div>
                                <div
                                    style={{
                                        display: 'flex',
                                        'flex-direction': 'row',
                                        gap: 'var(--p-space-m)'
                                    }}
                                >
                                    <Slider
                                        value={sliderVal()}
                                        onValueChange={setSliderVal}
                                        min={0}
                                        max={100}
                                        orientation="vertical"
                                    />
                                    <Slider
                                        defaultValue={50}
                                        min={0}
                                        max={100}
                                        orientation="vertical"
                                        disabled
                                    />
                                    <Slider
                                        defaultValue={50}
                                        step={25}
                                        min={0}
                                        max={100}
                                        orientation="vertical"
                                    />
                                    <Slider
                                        defaultValue={50}
                                        step={25}
                                        min={0}
                                        max={100}
                                        orientation="vertical"
                                        disabled
                                    />
                                </div>
                            </div>
                        </div>
                    </div>
                </section>

                {/* Overlays & Feedback */}
                <section style={{ 'margin-bottom': 'var(--p-space-3xl)' }}>
                    <h2
                        style={{
                            'margin-bottom': 'var(--p-space-l)',
                            'border-bottom': '1px solid var(--border-subtle)',
                            'padding-bottom': 'var(--p-space-s)'
                        }}
                    >
                        4. Overlay & Feedback
                    </h2>
                    <div
                        style={{
                            display: 'grid',
                            'grid-template-columns': 'repeat(2, 1fr)',
                            gap: 'var(--p-space-xl)'
                        }}
                    >
                        <div
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: 'var(--p-space-m)'
                            }}
                        >
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Modal Variants
                            </h3>
                            <div
                                style={{
                                    display: 'flex',
                                    'flex-wrap': 'wrap',
                                    gap: 'var(--p-space-s)'
                                }}
                            >
                                <Button onClick={() => setModalSmOpen(true)}>Small</Button>
                                <Button onClick={() => setModalMdOpen(true)}>Medium</Button>
                                <Button onClick={() => setModalLgOpen(true)}>Large</Button>
                                <Button onClick={() => setModalXlOpen(true)}>Extra Large</Button>
                                <Button onClick={() => setModalFullOpen(true)}>Full Screen</Button>
                            </div>
                            <div
                                style={{
                                    display: 'flex',
                                    'flex-wrap': 'wrap',
                                    gap: 'var(--p-space-s)',
                                    'margin-top': 'var(--p-space-s)'
                                }}
                            >
                                <Button
                                    variant="destructive"
                                    size="sm"
                                    onClick={() => setConfirmDangerOpen(true)}
                                >
                                    Danger Confirm
                                </Button>
                                <Button
                                    variant="secondary"
                                    size="sm"
                                    onClick={() => setConfirmWarningOpen(true)}
                                >
                                    Warning Confirm
                                </Button>
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => setPromptOpen(true)}
                                >
                                    Prompt Input
                                </Button>
                            </div>

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Toast Variants
                            </h3>

                            <div
                                style={{
                                    display: 'flex',
                                    'flex-direction': 'column',
                                    gap: 'var(--p-space-m)'
                                }}
                            >
                                {/* Position Selector */}
                                <div style={{ 'max-width': '300px' }}>
                                    <label
                                        style={{
                                            display: 'block',
                                            'margin-bottom': 'var(--p-space-s)',
                                            'font-size': 'var(--p-font-size-s)',
                                            color: 'var(--text-secondary)'
                                        }}
                                    >
                                        Position
                                    </label>
                                    <Select
                                        value={toastPos()}
                                        onValueChange={val => setToastPos(val as any)}
                                        options={[
                                            { label: 'Top Left', value: 'top-left' },
                                            { label: 'Top Center', value: 'top-center' },
                                            { label: 'Top Right', value: 'top-right' },
                                            { label: 'Bottom Left', value: 'bottom-left' },
                                            { label: 'Bottom Center', value: 'bottom-center' },
                                            { label: 'Bottom Right', value: 'bottom-right' }
                                        ]}
                                        placeholder="Select position"
                                    />
                                </div>

                                {/* Type Variants */}
                                <div>
                                    <h4
                                        style={{
                                            margin: '0 0 var(--p-space-s) 0',
                                            'font-size': 'var(--p-font-size-s)',
                                            'font-weight': '600',
                                            color: 'var(--text-primary)'
                                        }}
                                    >
                                        Types
                                    </h4>
                                    <div
                                        style={{
                                            display: 'flex',
                                            gap: 'var(--p-space-s)',
                                            'flex-wrap': 'wrap'
                                        }}
                                    >
                                        <Button
                                            variant="outline"
                                            onClick={() => toast.default('Event created')}
                                        >
                                            Default
                                        </Button>
                                        <Button
                                            variant="outline"
                                            onClick={() =>
                                                toast.success('Changes saved successfully')
                                            }
                                        >
                                            Success
                                        </Button>
                                        <Button
                                            variant="outline"
                                            onClick={() => toast.error('Failed to delete item')}
                                        >
                                            Error
                                        </Button>
                                        <Button
                                            variant="outline"
                                            onClick={() => toast.warning('Low disk space warning')}
                                        >
                                            Warning
                                        </Button>
                                        <Button
                                            variant="outline"
                                            onClick={() => toast.info('New updates available')}
                                        >
                                            Info
                                        </Button>
                                    </div>
                                </div>

                                {/* Features */}
                                <div>
                                    <h4
                                        style={{
                                            margin: '0 0 var(--p-space-s) 0',
                                            'font-size': 'var(--p-font-size-s)',
                                            'font-weight': '600',
                                            color: 'var(--text-primary)'
                                        }}
                                    >
                                        Features
                                    </h4>
                                    <div
                                        style={{
                                            display: 'flex',
                                            gap: 'var(--p-space-s)',
                                            'flex-wrap': 'wrap'
                                        }}
                                    >
                                        <Button
                                            variant="outline"
                                            onClick={() =>
                                                toast.success('File uploaded', {
                                                    description: 'image-01.png (2.4MB)'
                                                })
                                            }
                                        >
                                            With Description
                                        </Button>
                                        <Button
                                            variant="outline"
                                            onClick={() =>
                                                toast.default('Update installed', {
                                                    action: {
                                                        label: 'Restart',
                                                        onClick: () =>
                                                            toast.success('Restarting...')
                                                    }
                                                })
                                            }
                                        >
                                            With Action
                                        </Button>
                                        <Button
                                            variant="outline"
                                            onClick={() =>
                                                toast.warning('System update required', {
                                                    dismissible: false
                                                })
                                            }
                                        >
                                            Not Dismissible
                                        </Button>
                                        {/* <Button variant="outline" onClick={() => toast.default("Long duration toast...", { duration: 10000 })}> */}
                                        <Button
                                            variant="outline"
                                            onClick={() =>
                                                toast.default('Long duration toast...', {
                                                    duration: 1000000
                                                })
                                            }
                                        >
                                            Long Duration (10s)
                                        </Button>
                                    </div>
                                </div>
                            </div>
                        </div>

                        <div
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: 'var(--p-space-m)'
                            }}
                        >
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Tooltips
                            </h3>
                            <div
                                style={{
                                    display: 'flex',
                                    gap: 'var(--p-space-m)',
                                    'align-items': 'center'
                                }}
                            >
                                <Tooltip content="Helper text info" position="top">
                                    <Button variant="ghost" size="icon-sm">
                                        <Info size={16} />
                                    </Button>
                                </Tooltip>
                                <span>Top</span>
                                <Tooltip content="Helper text info" position="bottom">
                                    <Button variant="ghost" size="icon-sm">
                                        <Info size={16} />
                                    </Button>
                                </Tooltip>
                                <span>Bottom</span>
                                <Tooltip content="Helper text info" position="left">
                                    <Button variant="ghost" size="icon-sm">
                                        <Info size={16} />
                                    </Button>
                                </Tooltip>
                                <span>Left</span>
                                <Tooltip content="Helper text info" position="right">
                                    <Button variant="ghost" size="icon-sm">
                                        <Info size={16} />
                                    </Button>
                                </Tooltip>
                                <span>Right</span>
                            </div>

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Popovers
                            </h3>
                            <div style={{ width: '200px' }}>
                                <Popover
                                    trigger={<Button variant="outline">Trigger Popover</Button>}
                                >
                                    <h4>Popover Content</h4>
                                    <p
                                        style={{
                                            'font-size': 'var(--p-font-size-xs)',
                                            color: 'var(--text-secondary)'
                                        }}
                                    >
                                        This is inside a popover.
                                    </p>
                                </Popover>
                            </div>

                            <hr
                                style={{
                                    margin: 'var(--p-space-m) 0',
                                    border: 'none',
                                    'border-top': '1px solid var(--border-subtle)'
                                }}
                            />

                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Alerts
                            </h3>
                            <div
                                style={{
                                    display: 'flex',
                                    'flex-direction': 'column',
                                    gap: 'var(--p-space-m)',
                                    'min-width': '300px'
                                }}
                            >
                                <Alert variant="default" title="Default Alert">
                                    System update available.
                                </Alert>
                                <Alert variant="info" title="Info Alert">
                                    System update available.
                                </Alert>
                                <Alert variant="success" title="Success Alert">
                                    System update available.
                                </Alert>
                                <Alert variant="warning" title="Warning Alert">
                                    System update available.
                                </Alert>
                                <Alert variant="destructive" title="Error Alert">
                                    Failed to connect.
                                </Alert>
                            </div>
                        </div>
                    </div>
                </section>

                {/* Data Display */}
                <section style={{ 'margin-bottom': 'var(--p-space-3xl)' }}>
                    <h2
                        style={{
                            'margin-bottom': 'var(--p-space-l)',
                            'border-bottom': '1px solid var(--border-subtle)',
                            'padding-bottom': 'var(--p-space-s)'
                        }}
                    >
                        5. Data Display
                    </h2>
                    <div
                        style={{
                            display: 'grid',
                            'grid-template-columns': 'repeat(3, 1fr)',
                            gap: 'var(--p-space-xl)'
                        }}
                    >
                        <div
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: 'var(--p-space-m)'
                            }}
                        >
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Badges
                            </h3>
                            <div style={{ display: 'flex', gap: 'var(--p-space-s)' }}>
                                <Badge variant="default" size="sm">
                                    Default
                                </Badge>
                                <Badge variant="secondary" size="sm">
                                    Secondary
                                </Badge>
                                <Badge variant="outline" size="sm">
                                    Outline
                                </Badge>
                                <Badge variant="error" size="sm">
                                    Error
                                </Badge>
                                <Badge variant="warning" size="sm">
                                    Warning
                                </Badge>
                                <Badge variant="success" size="sm">
                                    Success
                                </Badge>
                                <Badge variant="info" size="sm">
                                    Info
                                </Badge>
                            </div>
                            <div style={{ display: 'flex', gap: 'var(--p-space-s)' }}>
                                <Badge variant="default" size="md">
                                    Default
                                </Badge>
                                <Badge variant="secondary" size="md">
                                    Secondary
                                </Badge>
                                <Badge variant="outline" size="md">
                                    Outline
                                </Badge>
                                <Badge variant="error" size="md">
                                    Error
                                </Badge>
                                <Badge variant="warning" size="md">
                                    Warning
                                </Badge>
                                <Badge variant="success" size="md">
                                    Success
                                </Badge>
                                <Badge variant="info" size="md">
                                    Info
                                </Badge>
                            </div>
                        </div>

                        <div
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: 'var(--p-space-s)'
                            }}
                        >
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Progress
                            </h3>

                            <div
                                style={{
                                    width: '200px',
                                    display: 'flex',
                                    'flex-direction': 'column',
                                    gap: 'var(--p-space-s)'
                                }}
                            >
                                <ProgressBar
                                    size="lg"
                                    value={sliderVal()}
                                    max={100}
                                    showLabel={true}
                                    label="Loading..."
                                />
                                <ProgressBar
                                    size="md"
                                    value={sliderVal()}
                                    max={100}
                                    showLabel={true}
                                    label="Loading..."
                                />
                                <ProgressBar
                                    size="sm"
                                    value={sliderVal()}
                                    max={100}
                                    showLabel={true}
                                    label="Loading..."
                                />

                                <ProgressBar
                                    size="md"
                                    value={sliderVal()}
                                    max={100}
                                    label="Loading..."
                                    indeterminate={true}
                                />
                            </div>
                        </div>
                        <div
                            style={{
                                display: 'flex',
                                'flex-direction': 'column',
                                gap: 'var(--p-space-s)'
                            }}
                        >
                            <h3
                                style={{
                                    'margin-bottom': 'var(--p-space-m)',
                                    color: 'var(--text-secondary)'
                                }}
                            >
                                Keyboard Shortcuts
                            </h3>
                            <span>
                                <Kbd>Cmd</Kbd> <Kbd>K</Kbd>
                            </span>
                            <span>
                                <Kbd>Shift</Kbd> <Kbd>A</Kbd>
                            </span>
                        </div>
                    </div>
                </section>
            </div>

            {/* Modal Examples */}
            <Modal
                isOpen={modalSmOpen()}
                onClose={() => setModalSmOpen(false)}
                title="Small Modal"
                size="sm"
                footer={
                    <div
                        style={{
                            display: 'flex',
                            'justify-content': 'flex-end',
                            gap: 'var(--p-space-s)'
                        }}
                    >
                        <Button variant="ghost" onClick={() => setModalSmOpen(false)}>
                            Close
                        </Button>
                        <Button onClick={() => setModalSmOpen(false)}>Okay</Button>
                    </div>
                }
            >
                <p>This is a small modal dialog content.</p>
            </Modal>

            <Modal
                isOpen={modalMdOpen()}
                onClose={() => setModalMdOpen(false)}
                title="Medium Modal (Default)"
                size="md"
                footer={
                    <div
                        style={{
                            display: 'flex',
                            'justify-content': 'flex-end',
                            gap: 'var(--p-space-s)'
                        }}
                    >
                        <Button variant="ghost" onClick={() => setModalMdOpen(false)}>
                            Cancel
                        </Button>
                        <Button onClick={() => setModalMdOpen(false)}>Confirm</Button>
                    </div>
                }
            >
                <div style={{ padding: 'var(--p-space-s) 0' }}>
                    <p>
                        This mimics a standard dialog in the application with <code>title</code>,{' '}
                        <code>children</code>, and <code>footer</code> props explicitly defined.
                    </p>
                    <div style={{ 'margin-top': 'var(--p-space-m)' }}>
                        <Input placeholder="Example input inside modal" />
                    </div>
                </div>
            </Modal>

            <Modal
                isOpen={modalLgOpen()}
                onClose={() => setModalLgOpen(false)}
                title="Large Modal"
                size="lg"
            >
                <p>This is a large modal suitable for more complex forms or content.</p>
                <div
                    style={{
                        height: '200px',
                        background: 'var(--bg-surface-1)',
                        'margin-top': 'var(--p-space-m)',
                        display: 'flex',
                        'align-items': 'center',
                        'justify-content': 'center'
                    }}
                >
                    Placeholder Content Area
                </div>
            </Modal>

            <Modal
                isOpen={modalXlOpen()}
                onClose={() => setModalXlOpen(false)}
                title="Extra Large Modal"
                size="xl"
            >
                <p>This is an extra large modal for wide content like tables or previews.</p>
            </Modal>

            <Modal
                isOpen={modalFullOpen()}
                onClose={() => setModalFullOpen(false)}
                title="Full Screen Modal"
                size="full"
            >
                <p>This modal takes up the entire screen.</p>
                <Button
                    variant="outline"
                    onClick={() => setModalFullOpen(false)}
                    style={{ 'margin-top': 'var(--p-space-m)' }}
                >
                    Close Fullscreen
                </Button>
            </Modal>

            {/* Specialized Modals */}
            <ConfirmModal
                isOpen={confirmDangerOpen()}
                onClose={() => setConfirmDangerOpen(false)}
                onConfirm={() => {
                    toast.success('Deleted!');
                }}
                title="Delete Item?"
                message="Are you sure you want to delete this item? This action cannot be undone."
                confirmText="Delete"
                kind="danger"
            />

            <ConfirmModal
                isOpen={confirmWarningOpen()}
                onClose={() => setConfirmWarningOpen(false)}
                onConfirm={() => {
                    toast.info('Proceeded with caution.');
                }}
                title="Warning"
                message="This might have side effects. Do you wish to proceed?"
                confirmText="Proceed"
                kind="warning"
            />

            <PromptModal
                isOpen={promptOpen()}
                onClose={() => setPromptOpen(false)}
                onConfirm={val => {
                    toast.success(`Input received: ${val}`);
                }}
                title="Enter Value"
                description="Please enter your unique username below to continue."
                placeholder="Type 'confirm' to pass"
                initialValue=""
                validate={val => (val !== 'confirm' ? "You must type 'confirm'" : undefined)}
                required
            />
        </div>
    );
};

// Helper
const ColorSwatch: Component<{ name: string; val: string; border?: boolean }> = props => {
    return (
        <div style={{ display: 'flex', 'flex-direction': 'column', gap: '8px' }}>
            <div
                style={{
                    width: '80px',
                    height: '80px',
                    'background-color': props.val,
                    'border-radius': 'var(--radius-m)',
                    border: props.border ? '1px solid var(--border-default)' : 'none'
                }}
            />
            <span style={{ 'font-size': 'var(--p-font-size-xs)' }}>{props.name}</span>
        </div>
    );
};
