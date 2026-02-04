import { Component, createSignal, onMount } from "solid-js";
import { Button } from "../../ui/Button";
import { RadioGroup, RadioGroupItem } from "../../ui/RadioGroup";
import { Input } from "../../ui/Input";
import { Badge } from "../../ui/Badge";
import { Switch } from "../../ui/Switch";
import { Slider } from "../../ui/Slider";
import { Alert } from "../../ui/Alert";
import { Select } from "../../ui/Select";
import { Checkbox } from "../../ui/Checkbox";
import { ToggleGroup, ToggleGroupItem } from "../../ui/ToggleGroup";
import { ProgressBar } from "../../ui/ProgressBar";
import { Tooltip } from "../../ui/Tooltip";
import { Popover } from "../../ui/Popover";
import { Modal } from "../../ui/Modal";
import { Sonner, toast } from "../../ui/Sonner";
import { Kbd } from "../../ui/Kbd";
import { TagInput } from "../../ui/TagInput";
import { Search, Info, Bold, Italic, Underline, Sun, Moon, AlignCenter, AlignLeft, AlignRight } from "lucide-solid";
import "../../ui/sonner.css"; // Ensure sonner styles are loaded
import { Toggle } from "../../ui";

export const DesignSystemGuide: Component = () => {
  const [modalOpen, setModalOpen] = createSignal(false);
  const [sliderVal, setSliderVal] = createSignal(50);
  const [toggleGroupVal, setToggleGroupVal] = createSignal<string | null>("bold");
  const [tags, setTags] = createSignal<string[]>(["Design", "System"]);

  onMount(() => {
      window.dispatchEvent(new CustomEvent('app-ready'));
  });

  return (
    <div style={{ 
        "background-color": "var(--bg-page)", 
        "color": "var(--text-primary)",
        "min-height": "100vh",
        "padding": "var(--p-space-xl)",
        "font-family": "var(--p-font-main)",
        "overflow-y": "auto",
        "height": "100vh",
        "box-sizing": "border-box"
    }}>
        <div style={{ "max-width": "1200px", "margin": "0 auto" }}>
            <header style={{ "margin-bottom": "var(--p-space-2xl)", "border-bottom": "1px solid var(--border-default)", "padding-bottom": "var(--p-space-l)" }}>
                <h1 style={{ "font-size": "var(--p-font-size-3xl)", "margin-bottom": "var(--p-space-s)" }}>Elleven Design System</h1>
                <p style={{ "color": "var(--text-secondary)" }}>Component Guide & Style Reference</p>
            </header>

            <Sonner />

            {/* Typography & Colors Section */}
            <section style={{ "margin-bottom": "var(--p-space-3xl)" }}>
                <h2 style={{ "margin-bottom": "var(--p-space-l)", "border-bottom": "1px solid var(--border-subtle)", "padding-bottom": "var(--p-space-s)" }}>1. Tokens</h2>
                
                <div style={{ "display": "grid", "grid-template-columns": "1fr 1fr", "gap": "var(--p-space-2xl)" }}>
                    <div>
                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Colors</h3>
                        <div style={{ "display": "flex", "flex-wrap": "wrap", "gap": "var(--p-space-m)" }}>
                           <ColorSwatch name="Primary" val="var(--p-primary-500)" />
                           <ColorSwatch name="Success" val="var(--p-success-500)" />
                           <ColorSwatch name="Warning" val="var(--p-warning-500)" />
                           <ColorSwatch name="Error" val="var(--p-error-500)" />
                           <ColorSwatch name="Info" val="var(--p-info-500)" />
                        </div>
                         <div style={{ "display": "flex", "flex-wrap": "wrap", "gap": "var(--p-space-m)", "margin-top": "var(--p-space-m)" }}>
                           <ColorSwatch name="Bg Page" val="var(--bg-page)" border />
                           <ColorSwatch name="Surface 1" val="var(--bg-surface-1)" border />
                           <ColorSwatch name="Surface 2" val="var(--bg-surface-2)" border />
                           <ColorSwatch name="Surface 3" val="var(--bg-surface-3)" border />
                        </div>
                    </div>
                    <div>
                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Typography</h3>
                        <div>
                           <p style={{ "font-size": "var(--p-font-size-xxs)", "font-weight": "var(--p-font-weight-normal)", "line-height": "var(--p-line-height-xxs)" }}>Typography (XXS)</p>
                           <p style={{ "font-size": "var(--p-font-size-xs)", "font-weight": "var(--p-font-weight-normal)", "line-height": "var(--p-line-height-xs)" }}>Typography (XS)</p>
                           <p style={{ "font-size": "var(--p-font-size-s)", "font-weight": "var(--p-font-weight-normal)", "line-height": "var(--p-line-height-s)" }}>Typography (S)</p>
                           <p style={{ "font-size": "var(--p-font-size-m)", "font-weight": "var(--p-font-weight-normal)", "line-height": "var(--p-line-height-m)" }}>Typography (M)</p>
                           <p style={{ "font-size": "var(--p-font-size-l)", "font-weight": "var(--p-font-weight-normal)", "line-height": "var(--p-line-height-l)" }}>Typography (L)</p>
                           <p style={{ "font-size": "var(--p-font-size-xl)", "font-weight": "var(--p-font-weight-normal)", "line-height": "var(--p-line-height-xl)" }}>Typography (XL)</p>
                           <p style={{ "font-size": "var(--p-font-size-2xl)", "font-weight": "var(--p-font-weight-normal)", "line-height": "var(--p-line-height-2xl)" }}>Typography (2XL)</p>
                           <p style={{ "font-size": "var(--p-font-size-3xl)", "font-weight": "var(--p-font-weight-normal)", "line-height": "var(--p-line-height-3xl)" }}>Typography (3XL)</p>
                        </div>
                    </div>
                </div>
            </section>

            {/* Buttons */}
            <section style={{ "margin-bottom": "var(--p-space-3xl)" }}>
                <h2 style={{ "margin-bottom": "var(--p-space-l)", "border-bottom": "1px solid var(--border-subtle)", "padding-bottom": "var(--p-space-s)" }}>2. Buttons</h2>
                <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-l)" }}>
                    {/* Primary */}
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="primary" size="lg">Primary</Button>
                        <Button variant="primary" size="lg" disabled>Primary</Button>
                        <Button variant="primary" size="lg" loading>Loading</Button>
                        <Button variant="primary" size="lg" leftIcon={<Search />}>Primary</Button>
                        <Button variant="primary" size="lg" rightIcon={<Search />}>Primary</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="primary" size="md">Primary</Button>
                        <Button variant="primary" size="md" disabled>Primary</Button>
                        <Button variant="primary" size="md" loading>Loading</Button>
                        <Button variant="primary" size="md" leftIcon={<Search />}>Primary</Button>
                        <Button variant="primary" size="md" rightIcon={<Search />}>Primary</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="primary" size="sm">Primary</Button>
                        <Button variant="primary" size="sm" disabled>Primary</Button>
                        <Button variant="primary" size="sm" loading>Loading</Button>
                        <Button variant="primary" size="sm" leftIcon={<Search />}>Primary</Button>
                        <Button variant="primary" size="sm" rightIcon={<Search />}>Primary</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="primary" size="xs">Primary</Button>
                        <Button variant="primary" size="xs" disabled>Primary</Button>
                        <Button variant="primary" size="xs" loading>Loading</Button>
                        <Button variant="primary" size="xs" leftIcon={<Search />}>Primary</Button>
                        <Button variant="primary" size="xs" rightIcon={<Search />}>Primary</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="primary" size="icon"><Search /></Button>
                        <Button variant="primary" size="icon" disabled><Search /></Button>
                        <Button variant="primary" size="icon-sm"><Search /></Button>
                        <Button variant="primary" size="icon-sm" disabled><Search /></Button>
                        <Button variant="primary" size="icon-xs"><Search /></Button>
                        <Button variant="primary" size="icon-xs" disabled><Search /></Button>
                    </div>

                    {/* Secondary */}
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="secondary" size="lg">Secondary</Button>
                        <Button variant="secondary" size="lg" disabled>Secondary</Button>
                        <Button variant="secondary" size="lg" loading>Loading</Button>
                        <Button variant="secondary" size="lg" leftIcon={<Search />}>Secondary</Button>
                        <Button variant="secondary" size="lg" rightIcon={<Search />}>Secondary</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="secondary" size="md">Secondary</Button>
                        <Button variant="secondary" size="md" disabled>Secondary</Button>
                        <Button variant="secondary" size="md" loading>Loading</Button>
                        <Button variant="secondary" size="md" leftIcon={<Search />}>Secondary</Button>
                        <Button variant="secondary" size="md" rightIcon={<Search />}>Secondary</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="secondary" size="sm">Secondary</Button>
                        <Button variant="secondary" size="sm" disabled>Secondary</Button>
                        <Button variant="secondary" size="sm" loading>Loading</Button>
                        <Button variant="secondary" size="sm" leftIcon={<Search />}>Secondary</Button>
                        <Button variant="secondary" size="sm" rightIcon={<Search />}>Secondary</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="secondary" size="xs">Secondary</Button>
                        <Button variant="secondary" size="xs" disabled>Secondary</Button>
                        <Button variant="secondary" size="xs" loading>Loading</Button>
                        <Button variant="secondary" size="xs" leftIcon={<Search />}>Secondary</Button>
                        <Button variant="secondary" size="xs" rightIcon={<Search />}>Secondary</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="secondary" size="icon"><Search /></Button>
                        <Button variant="secondary" size="icon" disabled><Search /></Button>
                        <Button variant="secondary" size="icon-sm"><Search /></Button>
                        <Button variant="secondary" size="icon-sm" disabled><Search /></Button>
                        <Button variant="secondary" size="icon-xs"><Search /></Button>
                        <Button variant="secondary" size="icon-xs" disabled><Search /></Button>
                    </div>

                    {/* Outline */}
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="outline" size="lg">Outline</Button>
                        <Button variant="outline" size="lg" disabled>Outline</Button>
                        <Button variant="outline" size="lg" loading>Loading</Button>
                        <Button variant="outline" size="lg" leftIcon={<Search />}>Outline</Button>
                        <Button variant="outline" size="lg" rightIcon={<Search />}>Outline</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="outline" size="md">Outline</Button>
                        <Button variant="outline" size="md" disabled>Outline</Button>
                        <Button variant="outline" size="md" loading>Loading</Button>
                        <Button variant="outline" size="md" leftIcon={<Search />}>Outline</Button>
                        <Button variant="outline" size="md" rightIcon={<Search />}>Outline</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="outline" size="sm">Outline</Button>
                        <Button variant="outline" size="sm" disabled>Outline</Button>
                        <Button variant="outline" size="sm" loading>Loading</Button>
                        <Button variant="outline" size="sm" leftIcon={<Search />}>Outline</Button>
                        <Button variant="outline" size="sm" rightIcon={<Search />}>Outline</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="outline" size="xs">Outline</Button>
                        <Button variant="outline" size="xs" disabled>Outline</Button>
                        <Button variant="outline" size="xs" loading>Loading</Button>
                        <Button variant="outline" size="xs" leftIcon={<Search />}>Outline</Button>
                        <Button variant="outline" size="xs" rightIcon={<Search />}>Outline</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="outline" size="icon"><Search /></Button>
                        <Button variant="outline" size="icon" disabled><Search /></Button>
                        <Button variant="outline" size="icon-sm"><Search /></Button>
                        <Button variant="outline" size="icon-sm" disabled><Search /></Button>
                        <Button variant="outline" size="icon-xs"><Search /></Button>
                        <Button variant="outline" size="icon-xs" disabled><Search /></Button>
                    </div>

                    {/* Ghost */}
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost" size="lg">Ghost</Button>
                        <Button variant="ghost" size="lg" disabled>Ghost</Button>
                        <Button variant="ghost" size="lg" loading>Loading</Button>
                        <Button variant="ghost" size="lg" leftIcon={<Search />}>Ghost</Button>
                        <Button variant="ghost" size="lg" rightIcon={<Search />}>Ghost</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost" size="md">Ghost</Button>
                        <Button variant="ghost" size="md" disabled>Ghost</Button>
                        <Button variant="ghost" size="md" loading>Loading</Button>
                        <Button variant="ghost" size="md" leftIcon={<Search />}>Ghost</Button>
                        <Button variant="ghost" size="md" rightIcon={<Search />}>Ghost</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost" size="sm">Ghost</Button>
                        <Button variant="ghost" size="sm" disabled>Ghost</Button>
                        <Button variant="ghost" size="sm" loading>Loading</Button>
                        <Button variant="ghost" size="sm" leftIcon={<Search />}>Ghost</Button>
                        <Button variant="ghost" size="sm" rightIcon={<Search />}>Ghost</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost" size="xs">Ghost</Button>
                        <Button variant="ghost" size="xs" disabled>Ghost</Button>
                        <Button variant="ghost" size="xs" loading>Loading</Button>
                        <Button variant="ghost" size="xs" leftIcon={<Search />}>Ghost</Button>
                        <Button variant="ghost" size="xs" rightIcon={<Search />}>Ghost</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost" size="icon"><Search /></Button>
                        <Button variant="ghost" size="icon" disabled><Search /></Button>
                        <Button variant="ghost" size="icon-sm"><Search /></Button>
                        <Button variant="ghost" size="icon-sm" disabled><Search /></Button>
                        <Button variant="ghost" size="icon-xs"><Search /></Button>
                        <Button variant="ghost" size="icon-xs" disabled><Search /></Button>
                    </div>

                    {/* Ghost Destructive */}
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost-destructive" size="lg">Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="lg" disabled>Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="lg" loading>Loading</Button>
                        <Button variant="ghost-destructive" size="lg" leftIcon={<Search />}>Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="lg" rightIcon={<Search />}>Ghost Destructive</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost-destructive" size="md">Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="md" disabled>Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="md" loading>Loading</Button>
                        <Button variant="ghost-destructive" size="md" leftIcon={<Search />}>Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="md" rightIcon={<Search />}>Ghost Destructive</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost-destructive" size="sm">Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="sm" disabled>Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="sm" loading>Loading</Button>
                        <Button variant="ghost-destructive" size="sm" leftIcon={<Search />}>Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="sm" rightIcon={<Search />}>Ghost Destructive</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost-destructive" size="xs">Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="xs" disabled>Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="xs" loading>Loading</Button>
                        <Button variant="ghost-destructive" size="xs" leftIcon={<Search />}>Ghost Destructive</Button>
                        <Button variant="ghost-destructive" size="xs" rightIcon={<Search />}>Ghost Destructive</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="ghost-destructive" size="icon"><Search /></Button>
                        <Button variant="ghost-destructive" size="icon" disabled><Search /></Button>
                        <Button variant="ghost-destructive" size="icon-sm"><Search /></Button>
                        <Button variant="ghost-destructive" size="icon-sm" disabled><Search /></Button>
                        <Button variant="ghost-destructive" size="icon-xs"><Search /></Button>
                        <Button variant="ghost-destructive" size="icon-xs" disabled><Search /></Button>
                    </div>

                    {/* Destructive */}
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="destructive" size="lg">Destructive</Button>
                        <Button variant="destructive" size="lg" disabled>Destructive</Button>
                        <Button variant="destructive" size="lg" loading>Loading</Button>
                        <Button variant="destructive" size="lg" leftIcon={<Search />}>Destructive</Button>
                        <Button variant="destructive" size="lg" rightIcon={<Search />}>Destructive</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="destructive" size="md">Destructive</Button>
                        <Button variant="destructive" size="md" disabled>Destructive</Button>
                        <Button variant="destructive" size="md" loading>Loading</Button>
                        <Button variant="destructive" size="md" leftIcon={<Search />}>Destructive</Button>
                        <Button variant="destructive" size="md" rightIcon={<Search />}>Destructive</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="destructive" size="sm">Destructive</Button>
                        <Button variant="destructive" size="sm" disabled>Destructive</Button>
                        <Button variant="destructive" size="sm" loading>Loading</Button>
                        <Button variant="destructive" size="sm" leftIcon={<Search />}>Destructive</Button>
                        <Button variant="destructive" size="sm" rightIcon={<Search />}>Destructive</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="destructive" size="xs">Destructive</Button>
                        <Button variant="destructive" size="xs" disabled>Destructive</Button>
                        <Button variant="destructive" size="xs" loading>Loading</Button>
                        <Button variant="destructive" size="xs" leftIcon={<Search />}>Destructive</Button>
                        <Button variant="destructive" size="xs" rightIcon={<Search />}>Destructive</Button>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Button variant="destructive" size="icon"><Search /></Button>
                        <Button variant="destructive" size="icon" disabled><Search /></Button>
                        <Button variant="destructive" size="icon-sm"><Search /></Button>
                        <Button variant="destructive" size="icon-sm" disabled><Search /></Button>
                        <Button variant="destructive" size="icon-xs"><Search /></Button>
                        <Button variant="destructive" size="icon-xs" disabled><Search /></Button>
                    </div>
                </div>
            </section>

            {/* Toggle */}
            <section style={{ "margin-bottom": "var(--p-space-3xl)" }}>
                <h2 style={{ "margin-bottom": "var(--p-space-l)", "border-bottom": "1px solid var(--border-subtle)", "padding-bottom": "var(--p-space-s)" }}>3. Toggle</h2>
                <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-l)" }}>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Toggle aria-label="Toggle" pressed={true} size="lg"><Sun /> teste</Toggle>
                        <Toggle aria-label="Toggle" pressed={false} size="lg"><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} size="lg" disabled><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="lg"><Sun /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="lg"><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="lg" disabled><Moon /></Toggle>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Toggle aria-label="Toggle" pressed={true} size="md"><Sun /> teste</Toggle>
                        <Toggle aria-label="Toggle" pressed={false} size="md"><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} size="md" disabled><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="md"><Sun /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="md"><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="md" disabled><Moon /></Toggle>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                        <Toggle aria-label="Toggle" pressed={true} size="sm"><Sun /> teste</Toggle>
                        <Toggle aria-label="Toggle" pressed={false} size="sm"><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} size="sm" disabled><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="sm"><Sun /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="sm"><Moon /></Toggle>
                        <Toggle aria-label="Toggle" pressed={false} variant="outline" size="sm" disabled><Moon /></Toggle>
                    </div>
                </div>
            </section>

            {/* Toggle group */}
            <section style={{ "margin-bottom": "var(--p-space-3xl)" }}>
                <h2 style={{ "margin-bottom": "var(--p-space-l)", "border-bottom": "1px solid var(--border-subtle)", "padding-bottom": "var(--p-space-s)" }}>4. Toggle Group</h2>
                <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-l)" }}>
                    {/* size: xl */}
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                         {/* Single selection xl */}
                         <ToggleGroup type="single" orientation="horizontal" defaultValue="center" size="xl">
                            <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
                            <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
                            <ToggleGroupItem value="right" disabled><AlignRight /></ToggleGroupItem>
                         </ToggleGroup>
                         
                         {/* Multiple selection xl */}
                         <ToggleGroup type="multiple" orientation="horizontal" defaultValue={["bold"]} size="xl">
                            <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
                            <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
                         </ToggleGroup>


                         {/* Single selection lg */}
                         <ToggleGroup type="single" orientation="horizontal" defaultValue="center" size="lg">
                            <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
                            <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
                            <ToggleGroupItem value="right" disabled><AlignRight /></ToggleGroupItem>
                         </ToggleGroup>
                         
                         {/* Multiple selection lg */}
                         <ToggleGroup type="multiple" orientation="horizontal" defaultValue={["bold"]} size="lg">
                            <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
                            <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
                         </ToggleGroup>


                         {/* Single selection md */}
                         <ToggleGroup type="single" orientation="horizontal" defaultValue="center" size="md">
                            <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
                            <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
                            <ToggleGroupItem value="right" disabled><AlignRight /></ToggleGroupItem>
                         </ToggleGroup>
                         
                         {/* Multiple selection md */}
                         <ToggleGroup type="multiple" orientation="horizontal" defaultValue={["bold"]} size="md">
                            <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
                            <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
                         </ToggleGroup>


                         {/* Single selection sm */}
                         <ToggleGroup type="single" orientation="horizontal" defaultValue="center" size="sm">
                            <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
                            <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
                            <ToggleGroupItem value="right" disabled><AlignRight /></ToggleGroupItem>
                         </ToggleGroup>
                         
                         {/* Multiple selection sm */}
                         <ToggleGroup type="multiple" orientation="horizontal" defaultValue={["bold"]} size="sm">
                            <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
                            <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
                         </ToggleGroup>
                    </div>
                    <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                         {/* Single selection vertical xl */}
                         <ToggleGroup type="single" orientation="vertical" defaultValue="center" size="xl">
                            <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
                            <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
                            <ToggleGroupItem value="right" disabled><AlignRight /></ToggleGroupItem>
                         </ToggleGroup>

                         {/* Multiple selection vertical xl */}
                         <ToggleGroup type="multiple" orientation="vertical" defaultValue={["bold"]} size="xl">
                            <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
                            <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
                         </ToggleGroup>

                         {/* Single selection vertical lg */}
                         <ToggleGroup type="single" orientation="vertical" defaultValue="center" size="lg">
                            <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
                            <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
                            <ToggleGroupItem value="right" disabled><AlignRight /></ToggleGroupItem>
                         </ToggleGroup>

                         {/* Multiple selection vertical lg */}
                         <ToggleGroup type="multiple" orientation="vertical" defaultValue={["bold"]} size="lg">
                            <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
                            <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
                         </ToggleGroup>

                         {/* Single selection vertical md */}
                         <ToggleGroup type="single" orientation="vertical" defaultValue="center" size="md">
                            <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
                            <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
                            <ToggleGroupItem value="right" disabled><AlignRight /></ToggleGroupItem>
                         </ToggleGroup>

                         {/* Multiple selection vertical md */}
                         <ToggleGroup type="multiple" orientation="vertical" defaultValue={["bold"]} size="md">
                            <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
                            <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
                         </ToggleGroup>

                         {/* Single selection vertical sm */}
                         <ToggleGroup type="single" orientation="vertical" defaultValue="center" size="sm">
                            <ToggleGroupItem value="left"><AlignLeft /></ToggleGroupItem>
                            <ToggleGroupItem value="center"><AlignCenter /></ToggleGroupItem>
                            <ToggleGroupItem value="right" disabled><AlignRight /></ToggleGroupItem>
                         </ToggleGroup>

                         {/* Multiple selection vertical sm */}
                         <ToggleGroup type="multiple" orientation="vertical" defaultValue={["bold"]} size="sm">
                            <ToggleGroupItem value="bold"><Bold /></ToggleGroupItem>
                            <ToggleGroupItem value="italic"><Italic /></ToggleGroupItem>
                         </ToggleGroup>
                    </div>
                </div>
            </section>

             {/* Form Elements */}
             <section style={{ "margin-bottom": "var(--p-space-3xl)" }}>
                <h2 style={{ "margin-bottom": "var(--p-space-l)", "border-bottom": "1px solid var(--border-subtle)", "padding-bottom": "var(--p-space-s)" }}>3. Form Elements</h2>
                <div style={{ "display": "grid", "grid-template-columns": "repeat(3, 1fr)", "gap": "var(--p-space-xl)" }}>
                     <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-m)" }}>
                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Inputs with left icons</h3>
                        <Input placeholder="Standard Input..." size="lg" leftIcon={<Search />} />
                        <Input placeholder="Standard Input..." size="md" leftIcon={<Search />} />
                        <Input placeholder="Standard Input..." size="sm" leftIcon={<Search />} />

                        <hr style={{ "margin": "var(--p-space-m) 0", "border": "none", "border-top": "1px solid var(--border-subtle)" }} />

                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Selects with left icons</h3>
                        <Select 
                            options={[
                                {label: "Item 1", value: "1"},
                                {label: "Item 2", value: "2"},
                                {label: "Item 3", value: "3"}
                            ]}
                            placeholder="Select an item (lg)"
                            value={undefined}
                            onValueChange={() => {}}
                            size="lg"
                            leftIcon={<Search />}
                        />
                        <Select 
                            options={[
                                {label: "Item 1", value: "1"},
                                {label: "Item 2", value: "2"},
                                {label: "Item 3", value: "3"}
                            ]}
                            placeholder="Select an item (md)"
                            value={undefined}
                            onValueChange={() => {}}
                            size="md"
                            leftIcon={<Search />}
                        />
                        <Select 
                            options={[
                                {label: "Item 1", value: "1"},
                                {label: "Item 2", value: "2"},
                                {label: "Item 3", value: "3"}
                            ]}
                            placeholder="Select an item (sm)"
                            value={undefined}
                            onValueChange={() => {}}
                            size="sm"
                            leftIcon={<Search />}
                        />

                        <hr style={{ "margin": "var(--p-space-m) 0", "border": "none", "border-top": "1px solid var(--border-subtle)" }} />

                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Tag Input</h3>
                        <TagInput 
                            value={tags().map((t, i) => ({ id: i, label: t }))} 
                            onChange={(vals) => setTags(vals.map(v => v.label))} 
                            onCreate={(t) => setTags([...tags(), t])}
                        />
                        
                        <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                           <Switch checked={true} />
                            <Sun size={14} />
                           <Checkbox checked={true} label="Checkbox" />
                        </div>
                         <RadioGroup 
                            value="a"
                            onValueChange={() => {}}
                            name="demo-radio"
                         >
                            <RadioGroupItem value="a" label="Option A" />
                            <RadioGroupItem value="b" label="Option B" />
                         </RadioGroup>
                     </div>
                     <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-m)" }}>
                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Inputs with right icons</h3>
                        <Input placeholder="Standard Input..." size="lg" rightIcon={<Search />} />
                        <Input placeholder="Standard Input..." size="md" rightIcon={<Search />} />
                        <Input placeholder="Standard Input..." size="sm" rightIcon={<Search />} />

                        <hr style={{ "margin": "var(--p-space-m) 0", "border": "none", "border-top": "1px solid var(--border-subtle)" }} />

                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Selects with right icons</h3>
                        <Select 
                            options={[
                                {label: "Item 1", value: "1"},
                                {label: "Item 2", value: "2"},
                                {label: "Item 3", value: "3"}
                            ]}
                            placeholder="Select an item (lg)"
                            value={undefined}
                            onValueChange={() => {}}
                            size="lg"
                            rightIcon={<Search />}
                        />
                        <Select 
                            options={[
                                {label: "Item 1", value: "1"},
                                {label: "Item 2", value: "2"},
                                {label: "Item 3", value: "3"}
                            ]}
                            placeholder="Select an item (md)"
                            value={undefined}
                            onValueChange={() => {}}
                            size="md"
                            rightIcon={<Search />}
                        />
                        <Select 
                            options={[
                                {label: "Item 1", value: "1"},
                                {label: "Item 2", value: "2"},
                                {label: "Item 3", value: "3"}
                            ]}
                            placeholder="Select an item (sm)"
                            value={undefined}
                            onValueChange={() => {}}
                            size="sm"
                            rightIcon={<Search />}
                        />

                        <hr style={{ "margin": "var(--p-space-m) 0", "border": "none", "border-top": "1px solid var(--border-subtle)" }} />

                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Slider</h3>
                        <label>Slider ({Math.round(sliderVal())})</label>
                        <div style={{ "display": "flex", "flex-direction": "row", "gap": "var(--p-space-m)" }}>
                            <div style={{ "display": "flex", width: "100%", "flex-direction": "column", "gap": "var(--p-space-m)" }}>
                                <Slider value={sliderVal()} onValueChange={setSliderVal} min={0} max={100} />
                                <Slider value={50} min={0} max={100} disabled />
                                <Slider value={50} step={25} min={0} max={100}  />
                                <Slider value={50} step={25} min={0} max={100} disabled />
                            </div>
                            <div style={{ "display": "flex", "flex-direction": "row", "gap": "var(--p-space-m)" }}>
                                <Slider value={sliderVal()} onValueChange={setSliderVal} min={0} max={100} orientation="vertical" />
                                <Slider value={50} min={0} max={100} orientation="vertical" disabled />
                                <Slider value={50} step={25} min={0} max={100} orientation="vertical" />
                                <Slider value={50} step={25} min={0} max={100} orientation="vertical" disabled />
                            </div>
                        </div>
                     </div>
                     <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-m)" }}>
                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Inputs with both icons and disabled</h3>
                        <Input placeholder="Standard Input..." size="lg" leftIcon={<Search />} rightIcon={<Search />} disabled />
                        <Input placeholder="Standard Input..." size="md" leftIcon={<Search />} rightIcon={<Search />} disabled />
                        <Input placeholder="Standard Input..." size="sm" leftIcon={<Search />} rightIcon={<Search />} disabled />
                        
                        <hr style={{ "margin": "var(--p-space-m) 0", "border": "none", "border-top": "1px solid var(--border-subtle)" }} />

                        <h3 style={{ "margin-bottom": "var(--p-space-m)", "color": "var(--text-secondary)" }}>Selects with both icons and disabled</h3>
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
                     </div>
                </div>
            </section>

            {/* Overlays & Feedback */}
             <section style={{ "margin-bottom": "var(--p-space-3xl)" }}>
                <h2 style={{ "margin-bottom": "var(--p-space-l)", "border-bottom": "1px solid var(--border-subtle)", "padding-bottom": "var(--p-space-s)" }}>4. Overlay & Feedback</h2>
                <div style={{ "display": "flex", "gap": "var(--p-space-l)", "flex-wrap": "wrap" }}>
                     <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-m)" }}>
                        <Button onClick={() => setModalOpen(true)}>Open Modal</Button>
                        <div style={{ "display": "flex", "gap": "var(--p-space-s)" }}>
                             <Button variant="outline" onClick={() => toast.success("Operation successful!")}>Toast Success</Button>
                             <Button variant="outline" onClick={() => toast.error("Something went wrong")}>Toast Error</Button>
                        </div>
                     </div>

                     <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-m)" }}>
                        <div style={{ "display": "flex", "gap": "var(--p-space-m)", "align-items": "center" }}>
                            <Tooltip content="Helper text info">
                                <Button variant="ghost" size="icon-sm"><Info size={16} /></Button>
                            </Tooltip>
                            <span>Hover icon for Tooltip</span>
                        </div>

                         <Popover trigger={<Button variant="outline">Trigger Popover</Button>}>
                             <div style={{ "width": "200px" }}>
                                 <h4>Popover Content</h4>
                                 <p style={{ "font-size": "var(--p-font-size-xs)", "color": "var(--text-secondary)" }}>This is inside a popover.</p>
                             </div>
                         </Popover>
                     </div>
                     
                     <div style={{ "display": "flex", "flex-direction": "column", "gap": "var(--p-space-m)", "min-width": "300px" }}>
                        <Alert variant="info" title="Info Alert">System update available.</Alert>
                        <Alert variant="destructive" title="Error Alert">Failed to connect.</Alert>
                     </div>
                </div>
            </section>

             {/* Data Display */}
            <section style={{ "margin-bottom": "var(--p-space-3xl)" }}>
                <h2 style={{ "margin-bottom": "var(--p-space-l)", "border-bottom": "1px solid var(--border-subtle)", "padding-bottom": "var(--p-space-s)" }}>5. Data Display</h2>
                <div style={{ "display": "flex", "gap": "var(--p-space-xl)", "align-items": "center" }}>
                    <div style={{ "display": "flex", "gap": "var(--p-space-s)" }}>
                        <Badge variant="default">Default</Badge>
                        <Badge variant="secondary">Secondary</Badge>
                        <Badge variant="outline">Outline</Badge>
                        <Badge variant="error">Destructive</Badge>
                    </div>
                    <div style={{ "width": "200px" }}>
                        <label style={{ "margin-bottom": "8px", "display": "block" }}>Progress</label>
                        <ProgressBar value={sliderVal()} max={100} />
                    </div>
                         <div style={{ "display": "flex", "gap": "var(--p-space-s)", "align-items": "center" }}>
                             <span><Kbd>Cmd</Kbd> <Kbd>K</Kbd></span>
                             <span><Kbd>Shift</Kbd> <Kbd>A</Kbd></span>
                         </div>
                </div>

                <div style={{ "margin-top": "var(--p-space-l)" }}>
                   <ToggleGroup type="single" value={toggleGroupVal() || undefined} onValueChange={(v) => setToggleGroupVal(v)}>
                       <ToggleGroupItem value="bold"><Bold size={14} /></ToggleGroupItem>
                       <ToggleGroupItem value="italic"><Italic size={14} /></ToggleGroupItem>
                       <ToggleGroupItem value="underline"><Underline size={14} /></ToggleGroupItem>
                   </ToggleGroup>
                </div>
            </section>

        </div>

        <Modal isOpen={modalOpen()} onClose={() => setModalOpen(false)} title="Design System Modal" size="md">
            <div style={{ "padding": "var(--p-space-m)" }}>
                <p>This mimics a standard dialog in the application.</p>
                <div style={{ "margin-top": "var(--p-space-l)", "display": "flex", "justify-content": "flex-end", "gap": "var(--p-space-s)" }}>
                    <Button variant="ghost" onClick={() => setModalOpen(false)}>Cancel</Button>
                    <Button variant="primary" onClick={() => setModalOpen(false)}>Confirm</Button>
                </div>
            </div>
        </Modal>
    </div>
  );
};

// Helper
const ColorSwatch: Component<{name: string, val: string, border?: boolean}> = (props) => {
    return (
        <div style={{ "display": "flex", "flex-direction": "column", "gap": "8px" }}>
            <div style={{ 
                "width": "80px", 
                "height": "80px", 
                "background-color": props.val, 
                "border-radius": "var(--radius-m)",
                "border": props.border ? "1px solid var(--border-default)" : "none"
            }} />
            <span style={{ "font-size": "var(--p-font-size-xs)" }}>{props.name}</span>
        </div>
    )
}
