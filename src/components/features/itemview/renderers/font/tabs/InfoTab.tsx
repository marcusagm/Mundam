import { Component } from "solid-js";

interface InfoTabProps {
    fontFamily: string;
    src: string;
    name: string;
}

export const InfoTab: Component<InfoTabProps> = (props) => {
    return (
        <div class="font-info-container">
            <h3 class="font-info-title">Font Information</h3>
            
            <div class="font-info-content">
                <div class="font-info-row">
                    <span class="font-info-label">File Name</span>
                    <span class="font-info-value">{props.name}</span>
                </div>
                <div class="font-info-row">
                    <span class="font-info-label">Format</span>
                    <span class="font-info-value uppercase">{props.name.split('.').pop()}</span>
                </div>
                <div class="font-info-row">
                    <span class="font-info-label">Family Name</span>
                    <span class="font-info-value">{props.fontFamily.split('-')[2] || 'Unknown'}</span>
                </div>
                {/* 
                  To get real metadata (copyright, etc.) we'd need to parse the binary.
                  For now this is basic file info.
                */}
            </div>
        </div>
    );
};
