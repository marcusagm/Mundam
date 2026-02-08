import { render } from 'solid-js/web';
import { InputProvider } from './core/input';
import './styles/global.css';
import App from './App';
import { DesignSystemGuide } from './components/features/design';

const root = document.getElementById('root') as HTMLElement;

if (window.location.hash === '#design-system') {
    render(
        () => (
            <InputProvider>
                <DesignSystemGuide />
            </InputProvider>
        ),
        root
    );
} else {
    render(() => <App />, root);
}
