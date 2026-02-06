import { Component, For, createSignal, createMemo } from 'solid-js';
import { Star } from 'lucide-solid';
import './StarRating.css';

interface StarRatingProps {
    rating: number;
    onChange?: (rating: number) => void;
    max?: number;
    readOnly?: boolean;
}

export const StarRating: Component<StarRatingProps> = props => {
    const max = props.max || 5;
    const [hoverRating, setHoverRating] = createSignal(0);

    const displayRating = createMemo(() => hoverRating() || props.rating);

    const handleMouseEnter = (val: number) => {
        if (!props.readOnly) setHoverRating(val);
    };

    const handleMouseLeave = () => {
        if (!props.readOnly) setHoverRating(0);
    };

    const handleClick = (val: number) => {
        if (!props.readOnly && props.onChange) {
            // Toggle off if same rating clicked
            props.onChange(props.rating === val ? 0 : val);
        }
    };

    return (
        <div
            class="star-rating"
            onMouseLeave={handleMouseLeave}
            role="slider"
            aria-label="Rating"
            aria-valuemin="0"
            aria-valuemax={max}
            aria-valuenow={props.rating}
        >
            <For each={Array.from({ length: max }, (_, i) => i + 1)}>
                {starValue => (
                    <button
                        type="button"
                        class="star-button"
                        classList={{
                            'is-filled': starValue <= displayRating(),
                            'is-active': starValue <= props.rating,
                            'is-readonly': props.readOnly
                        }}
                        onMouseEnter={() => handleMouseEnter(starValue)}
                        onClick={() => handleClick(starValue)}
                        aria-label={`${starValue} Stars`}
                    >
                        <Star
                            size={16}
                            fill={starValue <= displayRating() ? 'currentColor' : 'none'}
                        />
                    </button>
                )}
            </For>
        </div>
    );
};
