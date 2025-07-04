// Smooth scrolling for anchor links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
        e.preventDefault();
        const target = document.querySelector(this.getAttribute('href'));
        if (target) {
            target.scrollIntoView({
                behavior: 'smooth',
                block: 'start'
            });
        }
    });
});

// Add scroll effect to navbar
const navbar = document.querySelector('.navbar');
let lastScroll = 0;

window.addEventListener('scroll', () => {
    const currentScroll = window.pageYOffset;
    
    if (currentScroll > 100) {
        navbar.style.backgroundColor = 'rgba(26, 26, 26, 0.98)';
    } else {
        navbar.style.backgroundColor = 'rgba(26, 26, 26, 0.95)';
    }
    
    lastScroll = currentScroll;
});

// Fetch latest release version from GitHub
async function updateVersion() {
    try {
        const response = await fetch('https://api.github.com/repos/yourusername/p2pgo/releases/latest');
        if (response.ok) {
            const data = await response.json();
            const version = data.tag_name || 'v1.0.0';
            document.querySelectorAll('.version').forEach(el => {
                el.textContent = version;
            });
        }
    } catch (error) {
        console.log('Could not fetch version info');
    }
}

// Update download link with latest release
async function updateDownloadLink() {
    try {
        const response = await fetch('https://api.github.com/repos/yourusername/p2pgo/releases/latest');
        if (response.ok) {
            const data = await response.json();
            const dmgAsset = data.assets.find(asset => asset.name.endsWith('.dmg'));
            if (dmgAsset) {
                document.querySelectorAll('a[href*="download/P2PGo"]').forEach(link => {
                    link.href = dmgAsset.browser_download_url;
                });
            }
        }
    } catch (error) {
        console.log('Could not fetch download info');
    }
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
    updateVersion();
    updateDownloadLink();
    
    // Add animation to features on scroll
    const observerOptions = {
        threshold: 0.1,
        rootMargin: '0px 0px -100px 0px'
    };
    
    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
            }
        });
    }, observerOptions);
    
    // Observe feature cards
    document.querySelectorAll('.feature').forEach(feature => {
        feature.style.opacity = '0';
        feature.style.transform = 'translateY(20px)';
        feature.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
        observer.observe(feature);
    });
    
    // Observe steps
    document.querySelectorAll('.step').forEach((step, index) => {
        step.style.opacity = '0';
        step.style.transform = 'translateY(20px)';
        step.style.transition = `opacity 0.6s ease ${index * 0.1}s, transform 0.6s ease ${index * 0.1}s`;
        observer.observe(step);
    });
});

// Add download tracking (optional - requires analytics setup)
document.querySelectorAll('.btn-primary').forEach(btn => {
    if (btn.textContent.includes('Download')) {
        btn.addEventListener('click', () => {
            // Track download event
            console.log('Download clicked');
            // You can add Google Analytics or other tracking here
        });
    }
});